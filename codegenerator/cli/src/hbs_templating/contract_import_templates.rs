///A module used for flattening and dealing with tuples and nested
///tuples in event params
mod nested_params {
    use super::*;
    pub type TupleParamIndex = usize;

    ///Recursive Representation of param token. With reference to it's own index
    ///if it is a tuple
    enum NestedEventParam {
        Param(ethers::abi::EventParam),
        TupleParam(TupleParamIndex, Box<NestedEventParam>),
        Tuple(Vec<NestedEventParam>),
    }

    ///Constructs NestedEventParam from an ethers abi EventParam
    impl From<ethers::abi::EventParam> for NestedEventParam {
        fn from(event_input: ethers::abi::EventParam) -> Self {
            if let ParamType::Tuple(param_types) = event_input.kind {
                //in the tuple case return a Tuple tape with an array of inner
                //event params
                Self::Tuple(
                    param_types
                        .into_iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let event_input = ethers::abi::EventParam {
                                // Keep the same name as the event input name
                                name: event_input.name.clone(),
                                kind: p,
                                //Tuple fields can't be indexed
                                indexed: false,
                            };
                            //Recursively get the inner NestedEventParam type
                            Self::TupleParam(i, Box::new(Self::from(event_input)))
                        })
                        .collect(),
                )
            } else {
                Self::Param(event_input)
            }
        }
    }

    impl NestedEventParam {
        //Turns the recursive NestedEventParam structure into a vec of FlattenedEventParam structs
        //This is the internal function that takes an array as a second param. The public function
        //calls this with an empty vec.
        fn into_flattened_inputs_inner(
            &self,
            mut accessor_indexes: Vec<TupleParamIndex>,
        ) -> Vec<FlattenedEventParam> {
            match &self {
                Self::Param(e) => {
                    let accessor_indexes = if accessor_indexes.is_empty() {
                        None
                    } else {
                        Some(accessor_indexes)
                    };

                    vec![FlattenedEventParam {
                        event_param: e.clone(),
                        accessor_indexes,
                    }]
                }
                Self::TupleParam(i, arg_or_tuple) => {
                    accessor_indexes.push(*i);
                    arg_or_tuple.into_flattened_inputs_inner(accessor_indexes)
                }
                Self::Tuple(params) => params
                    .iter()
                    .flat_map(|param| param.into_flattened_inputs_inner(accessor_indexes.clone()))
                    .collect::<Vec<_>>(),
            }
        }

        //Public function that converts the NestedEventParam into a Vec of FlattenedEventParams
        //calls the internal function with an empty vec of accessor indexes
        pub fn into_flattened_inputs(&self) -> Vec<FlattenedEventParam> {
            self.into_flattened_inputs_inner(vec![])
        }
    }

    ///A flattened representation of an event param, meaning
    ///tuples/structs would broken into a single FlattenedEventParam for each
    ///param that it contains and include accessor indexes for where to find that param
    ///within its parent tuple/struct
    #[derive(Debug, Clone, PartialEq)]
    pub struct FlattenedEventParam {
        pub event_param: ethers::abi::EventParam,
        pub accessor_indexes: Option<Vec<TupleParamIndex>>,
    }

    impl FlattenedEventParam {
        ///Gets the key of the param for the entity representing thes event
        ///If this is not a tuple it will be the same as the "event_param_key"
        ///eg. MyEventEntity has a param called myTupleParam_1_2, where as the
        ///event_param_key is myTupleParam with accessor_indexes of [1, 2]
        ///In a JS template this would be myTupleParam[1][2] to get the value of the parameter
        pub fn get_entity_key(&self) -> CapitalizedOptions {
            let accessor_indexes_string = self.accessor_indexes.as_ref().map_or_else(
                //If there is no accessor_indexes this is an empty string
                || "".to_string(),
                |accessor_indexes| {
                    format!(
                        "_{}",
                        //join each index with "_"
                        //eg. _1_2 for a double nested tuple
                        accessor_indexes
                            .iter()
                            .map(|u| u.to_string())
                            .collect::<Vec<_>>()
                            .join("_")
                    )
                },
            );

            //Join the param name with the accessor_indexes_string
            //eg. myTupleParam_1_2 or myNonTupleParam if there are no accessor indexes
            format!("{}{}", self.event_param.name, accessor_indexes_string).to_capitalized_options()
        }

        ///Gets the event param "key" for the event type. Will be the same
        ///as the entity key if the type is not a tuple. In the case of a tuple
        ///entity key will append _0_1 for eg to represent thested param in a flat structure
        ///the event param key will not append this and will need to access that tuple at the given
        ///index
        pub fn get_event_param_key(&self) -> CapitalizedOptions {
            self.event_param.name.to_capitalized_options()
        }
    }

    ///Take an event, and if any param is a tuple type,
    ///it flattens it into an event with more params
    ///MyEvent(address myAddress, (uint256, bool) myTupleParam) ->
    ///MyEvent(address myAddress, uint256 myTupleParam_1, uint256 myTupleParam_2)
    ///This representation makes it easy to have single field conversions
    pub fn flatten_event_inputs(
        event_inputs: Vec<ethers::abi::EventParam>,
    ) -> Vec<FlattenedEventParam> {
        event_inputs
            .into_iter()
            .flat_map(|event_input| NestedEventParam::from(event_input).into_flattened_inputs())
            .collect()
    }
}

use super::hbs_dir_generator::HandleBarsDirGenerator;
use crate::{
    capitalization::{Capitalize, CapitalizedOptions},
    cli_args::clap_definitions::Language,
    config_parsing::{
        entity_parsing::{ethabi_type_to_field_type, Entity, Field, FieldType, Schema},
        system_config::{self, SystemConfig},
    },
    template_dirs::TemplateDirs,
};
use anyhow::{Context, Result};
use ethers::abi::ParamType;
use nested_params::{flatten_event_inputs, FlattenedEventParam, TupleParamIndex};
use serde::Serialize;
use std::path::PathBuf;

///The struct that houses all the details of each contract necessary for
///populating the contract import templates
#[derive(Serialize)]
pub struct AutoSchemaHandlerTemplate {
    contracts: Vec<Contract>,
}

impl Into<Schema> for AutoSchemaHandlerTemplate {
    fn into(self) -> Schema {
        let entities = self
            .contracts
            .into_iter()
            .flat_map(|c| {
                let schema: Schema = c.into();
                schema.entities
            })
            .collect();
        Schema { entities }
    }
}

#[derive(Serialize)]
pub struct Contract {
    name: CapitalizedOptions,
    events: Vec<Event>,
}

impl Contract {
    fn from_config_contract(contract: &system_config::Contract) -> Result<Self> {
        let events = contract
            .events
            .iter()
            .map(|event| Event::from_config_event(event))
            .collect::<Result<_>>()
            .context(format!(
                "Failed getting events for contract {}",
                contract.name
            ))?;

        Ok(Contract {
            name: contract.name.to_capitalized_options(),
            events,
        })
    }
}

impl Into<Schema> for Contract {
    fn into(self) -> Schema {
        let entities = self.events.into_iter().map(|e| e.into()).collect();
        Schema { entities }
    }
}

#[derive(Serialize)]
pub struct Event {
    name: CapitalizedOptions,
    params: Vec<Param>,
}

impl Event {
    fn from_config_event(e: &system_config::Event) -> Result<Self> {
        let params = flatten_event_inputs(e.event.inputs.clone())
            .into_iter()
            .map(|input| Param::from_event_param(input))
            .collect::<Result<_>>()
            .context(format!("Failed getting params for event {}", e.event.name))?;

        Ok(Event {
            name: e.event.name.to_capitalized_options(),
            params,
        })
    }
}

impl Into<Entity> for Event {
    fn into(self) -> Entity {
        let fields = self.params.into_iter().map(|p| p.into()).collect();
        Entity {
            name: self.name.original,
            fields,
        }
    }
}

///Param is used both in the context of an entity and an event for the generating
///schema and handlers.
#[derive(Serialize)]
pub struct Param {
    ///Event param name + index if its a tuple ie. myTupleParam_0_1 or just myRegularParam
    entity_key: CapitalizedOptions,
    ///Just the event param name accessible on the event type
    event_key: CapitalizedOptions,
    ///List of nested acessors so for a nested tuple Some([0, 1]) this can be used combined with
    ///the event key ie. event.params.myTupleParam[0][1]
    tuple_param_accessor_indexes: Option<Vec<TupleParamIndex>>,
    graphql_type: FieldType,
    is_eth_address: bool,
}

impl Param {
    fn from_event_param(flattened_event_param: FlattenedEventParam) -> Result<Self> {
        Ok(Param {
            entity_key: flattened_event_param.get_entity_key(),
            event_key: flattened_event_param.get_event_param_key(),
            tuple_param_accessor_indexes: flattened_event_param.accessor_indexes,
            graphql_type: ethabi_type_to_field_type(&flattened_event_param.event_param.kind)
                .context("converting eth event param to gql scalar")?,
            is_eth_address: flattened_event_param.event_param.kind == ParamType::Address,
        })
    }
}

impl Into<Field> for Param {
    fn into(self) -> Field {
        Field {
            name: self.entity_key.original,
            field_type: self.graphql_type,
            derived_from_field: None,
        }
    }
}

impl AutoSchemaHandlerTemplate {
    pub fn try_from(config: SystemConfig) -> Result<Self> {
        let contracts = config
            .get_contracts()
            .iter()
            .map(|c| Contract::from_config_contract(c))
            .collect::<Result<_>>()?;
        Ok(AutoSchemaHandlerTemplate { contracts })
    }

    pub fn generate_templates(&self, lang: &Language, project_root: &PathBuf) -> Result<()> {
        let template_dirs = TemplateDirs::new();

        let shared_dir = template_dirs
            .get_contract_import_shared_dir()
            .context("Failed getting shared contract import templates")?;

        let lang_dir = template_dirs
            .get_contract_import_lang_dir(lang)
            .context(format!("Failed getting {} contract import templates", lang))?;

        let hbs = HandleBarsDirGenerator::new(&lang_dir, &self, &project_root);
        let hbs_shared = HandleBarsDirGenerator::new(&shared_dir, &self, &project_root);
        hbs.generate_hbs_templates().context(format!(
            "Failed generating {} contract import templates",
            lang
        ))?;
        hbs_shared
            .generate_hbs_templates()
            .context("Failed generating shared contract import templates")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ethers::abi::{EventParam, HumanReadableParser};

    impl FlattenedEventParam {
        fn new(name: &str, kind: ParamType, indexed: bool, accessor_indexes: Vec<usize>) -> Self {
            let accessor_indexes = if accessor_indexes.is_empty() {
                None
            } else {
                Some(accessor_indexes)
            };

            Self {
                event_param: EventParam {
                    name: name.to_string(),
                    kind,
                    indexed,
                },
                accessor_indexes,
            }
        }
    }
    #[test]
    fn flatten_event_with_tuple() {
        let event_inputs = vec![
            EventParam {
                name: "user".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "myTupleParam".to_string(),
                kind: ParamType::Tuple(vec![ParamType::Uint(256), ParamType::Bool]),
                indexed: false,
            },
        ];

        let expected_flat_inputs = vec![
            FlattenedEventParam::new("user", ParamType::Address, false, vec![]),
            FlattenedEventParam::new("myTupleParam", ParamType::Uint(256), false, vec![0]),
            FlattenedEventParam::new("myTupleParam", ParamType::Bool, false, vec![1]),
        ];

        let actual_flat_inputs = flatten_event_inputs(event_inputs);
        assert_eq!(expected_flat_inputs, actual_flat_inputs);

        let expected_entity_keys: Vec<_> = vec!["user", "myTupleParam_0", "myTupleParam_1"]
            .into_iter()
            .map(|s| s.to_string().to_capitalized_options())
            .collect();

        let actual_entity_keys: Vec<_> = actual_flat_inputs
            .iter()
            .map(|f| f.get_entity_key())
            .collect();

        assert_eq!(expected_entity_keys, actual_entity_keys);
    }

    #[test]
    fn flatten_event_with_nested_tuple() {
        let event_inputs = vec![
            EventParam {
                name: "user".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "myTupleParam".to_string(),
                kind: ParamType::Tuple(vec![
                    ParamType::Tuple(vec![ParamType::Uint(8), ParamType::Uint(8)]),
                    ParamType::Bool,
                ]),
                indexed: false,
            },
        ];

        let expected_flat_inputs = vec![
            FlattenedEventParam::new("user", ParamType::Address, false, vec![]),
            FlattenedEventParam::new("myTupleParam", ParamType::Uint(8), false, vec![0, 0]),
            FlattenedEventParam::new("myTupleParam", ParamType::Uint(8), false, vec![0, 1]),
            FlattenedEventParam::new("myTupleParam", ParamType::Bool, false, vec![1]),
        ];
        let actual_flat_inputs = flatten_event_inputs(event_inputs);
        assert_eq!(expected_flat_inputs, actual_flat_inputs);

        let expected_entity_keys: Vec<_> = vec![
            "user",
            "myTupleParam_0_0",
            "myTupleParam_0_1",
            "myTupleParam_1",
        ]
        .into_iter()
        .map(|s| s.to_string().to_capitalized_options())
        .collect();

        let actual_entity_keys: Vec<_> = actual_flat_inputs
            .iter()
            .map(|f| f.get_entity_key())
            .collect();

        assert_eq!(expected_entity_keys, actual_entity_keys);
    }
}
