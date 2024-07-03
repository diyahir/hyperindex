use super::validation;
use crate::{constants::links, utils::normalized_list::NormalizedList};
use anyhow::Context;
use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::PathBuf};

type NetworkId = u64;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GlobalContract<T> {
    pub name: String,
    #[serde(flatten)]
    pub config: T,
}

pub type Addresses = NormalizedList<String>;

impl JsonSchema for Addresses {
    fn schema_name() -> Cow<'static, str> {
        "Addresses".into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        let t_schema = json_schema!({
          "anyOf": [
            String::json_schema(gen),
            usize::json_schema(gen),
          ]
        });
        json_schema!({
          "anyOf": [
            t_schema,
            {
              "type": "array",
              "items": t_schema
            }
          ]
        })
    }

    fn _schemars_private_is_option() -> bool {
        true
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct NetworkContract<T> {
    pub name: String,
    pub address: Addresses,
    #[serde(flatten)]
    //If this is "None" it should be expected that
    //there is a global config for the contract
    pub config: Option<T>,
}

pub mod evm {
    use super::{GlobalContract, NetworkContract, NetworkId};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
    #[schemars(
        title = "Envio Config Schema",
        description = "Schema for a YAML config for an envio indexer"
    )]
    pub struct HumanConfig {
        #[schemars(description = "Name of the project")]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Description of the project")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Ecosystem of the project.")]
        pub ecosystem: Option<EcosystemTag>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Custom path to config file")]
        pub schema: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "Global contract definitions that must contain all definitions except addresses. You can share a single handler/abi/event definitions for contracts across multiple chains."
        )]
        pub contracts: Option<Vec<GlobalContract<ContractConfig>>>,
        #[schemars(
            description = "Configuration of the blockchain networks that the project is deployed on."
        )]
        pub networks: Vec<Network>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "A flag to indicate if the indexer should use a single queue for all chains or a queue per chain (default: false)"
        )]
        pub unordered_multichain_mode: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "The event decoder to use for the indexer (default: hypersync-client)"
        )]
        pub event_decoder: Option<EventDecoder>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "A flag to indicate if the indexer should rollback to the last known valid block on a reorg (default: false)"
        )]
        pub rollback_on_reorg: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "A flag to indicate if the indexer should save the full history of events. This is useful for debugging but will increase the size of the database (default: false)"
        )]
        pub save_full_history: Option<bool>,
    }

    // Workaround for https://github.com/serde-rs/serde/issues/2231
    #[derive(Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
    #[serde(rename_all = "lowercase")]
    pub enum EcosystemTag {
        Evm,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
    #[serde(rename_all = "kebab-case")]
    pub enum EventDecoder {
        Viem,
        HypersyncClient,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    pub struct HypersyncConfig {
        pub url: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
    pub struct SyncConfigUnstable {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub initial_block_interval: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub backoff_multiplicative: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub acceleration_additive: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub interval_ceiling: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub backoff_millis: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub query_timeout_millis: Option<u32>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    #[allow(non_snake_case)] //Stop compiler warning for the double underscore in unstable__sync_config
    pub struct RpcConfig {
        pub url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub unstable__sync_config: Option<SyncConfigUnstable>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    pub struct Network {
        pub id: NetworkId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rpc_config: Option<RpcConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hypersync_config: Option<HypersyncConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub confirmed_block_threshold: Option<i32>,
        pub start_block: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end_block: Option<i32>,
        pub contracts: Vec<NetworkContract<ContractConfig>>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    pub struct ContractConfig {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub abi_file_path: Option<String>,
        pub handler: String,
        pub events: Vec<EventConfig>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct EventConfig {
        pub event: String,
    }

    impl EventConfig {
        pub fn event_string_from_abi_event(abi_event: &ethers::abi::Event) -> String {
            format!(
                "{}({}){}",
                abi_event.name,
                abi_event
                    .inputs
                    .iter()
                    .map(|input| {
                        let param_type = input.kind.to_string();
                        let indexed_keyword = if input.indexed { " indexed " } else { " " };
                        let param_name = input.name.clone();

                        format!("{}{}{}", param_type, indexed_keyword, param_name)
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                if abi_event.anonymous {
                    " anonymous"
                } else {
                    ""
                },
            )
        }
    }
}

pub mod fuel {
    use super::{GlobalContract, NetworkContract, NetworkId};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
    #[schemars(
        title = "Envio Config Schema",
        description = "Schema for a YAML config for an envio indexer"
    )]
    pub struct HumanConfig {
        #[schemars(description = "Name of the project")]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Description of the project")]
        pub description: Option<String>,
        #[schemars(description = "Ecosystem of the project.")]
        pub ecosystem: EcosystemTag,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Custom path to config file")]
        pub schema: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "Global contract definitions that must contain all definitions except addresses. You can share a single handler/abi/event definitions for contracts across multiple chains."
        )]
        pub contracts: Option<Vec<GlobalContract<ContractConfig>>>,
        #[schemars(
            description = "Configuration of the blockchain networks that the project is deployed on."
        )]
        pub networks: Vec<Network>,
    }

    // Workaround for https://github.com/serde-rs/serde/issues/2231
    #[derive(Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
    #[serde(rename_all = "lowercase")]
    pub enum EcosystemTag {
        Fuel,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    pub struct Network {
        pub id: NetworkId,
        pub start_block: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end_block: Option<i32>,
        pub contracts: Vec<NetworkContract<ContractConfig>>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    pub struct ContractConfig {
        pub abi_file_path: String,
        pub handler: String,
        pub events: Vec<EventConfig>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct EventConfig {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub log_id: Option<String>,
    }
}

fn strip_to_letters(string: &str) -> String {
    let mut pg_friendly_name = String::new();
    for c in string.chars() {
        if c.is_alphabetic() {
            pg_friendly_name.push(c);
        }
    }
    pg_friendly_name
}

pub fn deserialize_config_from_yaml(config_path: &PathBuf) -> anyhow::Result<evm::HumanConfig> {
    let config = std::fs::read_to_string(config_path).context(format!(
        "EE104: Failed to resolve config path {0}. Make sure you're in the correct directory and \
         that a config file with the name {0} exists",
        &config_path
            .to_str()
            .unwrap_or("unknown config file name path"),
    ))?;

    let mut deserialized_yaml: evm::HumanConfig =
        serde_yaml::from_str(&config).context(format!(
            "EE105: Failed to deserialize config. Visit the docs for more information {}",
            links::DOC_CONFIGURATION_FILE
        ))?;

    deserialized_yaml.name = strip_to_letters(&deserialized_yaml.name);

    // Validating the config file
    validation::validate_deserialized_config_yaml(config_path, &deserialized_yaml)?;

    Ok(deserialized_yaml)
}

#[cfg(test)]
mod tests {
    use super::{
        evm::{ContractConfig, EventDecoder, HumanConfig, Network},
        NetworkContract,
    };
    use crate::{config_parsing::human_config::fuel, utils::normalized_list::NormalizedList};
    use pretty_assertions::assert_eq;
    use schemars::{schema_for, Schema};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_evm_config_schema() {
        let config_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("npm/envio/evm.schema.json");
        let npm_schema: Schema =
            serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();

        let actual_schema = schema_for!(HumanConfig);

        // When the test is failing and you want to update the schema, uncomment this line
        // and paste the output into the schema file at npm/envio/evm.schema.json
        // println!("{}", serde_json::to_string_pretty(&actual_schema).unwrap());

        assert_eq!(npm_schema, actual_schema);
    }

    #[test]
    fn test_fuel_config_schema() {
        let config_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("npm/envio/fuel.schema.json");
        let npm_schema: Schema =
            serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();

        let actual_schema = schema_for!(fuel::HumanConfig);

        // When the test is failing and you want to update the schema, uncomment this line
        // and paste the output into the schema file at npm/envio/fuel.schema.json
        // println!("{}", serde_json::to_string_pretty(&actual_schema).unwrap());

        assert_eq!(npm_schema, actual_schema);
    }

    #[test]
    fn test_flatten_deserialize_local_contract() {
        let yaml = r#"
name: Contract1
handler: ./src/EventHandler.js
address: ["0x2E645469f354BB4F5c8a05B3b30A929361cf77eC"]
events: []
    "#;

        let deserialized: NetworkContract<ContractConfig> = serde_yaml::from_str(yaml).unwrap();
        let expected = NetworkContract {
            name: "Contract1".to_string(),
            address: NormalizedList::from(vec![
                "0x2E645469f354BB4F5c8a05B3b30A929361cf77eC".to_string()
            ]),
            config: Some(ContractConfig {
                abi_file_path: None,
                handler: "./src/EventHandler.js".to_string(),
                events: vec![],
            }),
        };

        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_flatten_deserialize_local_contract_with_no_address() {
        let yaml = r#"
name: Contract1
handler: ./src/EventHandler.js
events: []
    "#;

        let deserialized: NetworkContract<ContractConfig> = serde_yaml::from_str(yaml).unwrap();
        let expected = NetworkContract {
            name: "Contract1".to_string(),
            address: vec![].into(),
            config: Some(ContractConfig {
                abi_file_path: None,
                handler: "./src/EventHandler.js".to_string(),
                events: vec![],
            }),
        };

        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_flatten_deserialize_local_contract_with_single_address() {
        let yaml = r#"
name: Contract1
handler: ./src/EventHandler.js
address: "0x2E645469f354BB4F5c8a05B3b30A929361cf77eC"
events: []
    "#;

        let deserialized: NetworkContract<ContractConfig> = serde_yaml::from_str(yaml).unwrap();
        let expected = NetworkContract {
            name: "Contract1".to_string(),
            address: vec!["0x2E645469f354BB4F5c8a05B3b30A929361cf77eC".to_string()].into(),
            config: Some(ContractConfig {
                abi_file_path: None,
                handler: "./src/EventHandler.js".to_string(),
                events: vec![],
            }),
        };

        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_flatten_deserialize_global_contract() {
        let yaml = r#"
name: Contract1
address: ["0x2E645469f354BB4F5c8a05B3b30A929361cf77eC"]
    "#;

        let deserialized: NetworkContract<ContractConfig> = serde_yaml::from_str(yaml).unwrap();
        let expected = NetworkContract {
            name: "Contract1".to_string(),
            address: NormalizedList::from(vec![
                "0x2E645469f354BB4F5c8a05B3b30A929361cf77eC".to_string()
            ]),
            config: None,
        };

        assert_eq!(expected, deserialized);
    }

    #[test]
    fn deserialize_address() {
        let no_address = r#"null"#;
        let deserialized: NormalizedList<String> = serde_json::from_str(no_address).unwrap();
        assert_eq!(deserialized, NormalizedList::from(vec![]));

        let single_address = r#""0x123""#;
        let deserialized: NormalizedList<String> = serde_json::from_str(single_address).unwrap();
        assert_eq!(
            deserialized,
            NormalizedList::from(vec!["0x123".to_string()])
        );

        let multi_address = r#"["0x123", "0x456"]"#;
        let deserialized: NormalizedList<String> = serde_json::from_str(multi_address).unwrap();
        assert_eq!(
            deserialized,
            NormalizedList::from(vec!["0x123".to_string(), "0x456".to_string()])
        );
    }

    #[test]
    fn valid_name_conversion() {
        let name_with_space = super::strip_to_letters("My too lit to quit indexer");
        let expected_name_with_space = "Mytoolittoquitindexer";
        let name_with_special_chars = super::strip_to_letters("Myto@littoq$itindexer");
        let expected_name_with_special_chars = "Mytolittoqitindexer";
        let name_with_numbers = super::strip_to_letters("yes0123456789okay");
        let expected_name_with_numbers = "yesokay";
        assert_eq!(name_with_space, expected_name_with_space);
        assert_eq!(name_with_special_chars, expected_name_with_special_chars);
        assert_eq!(name_with_numbers, expected_name_with_numbers);
    }

    #[test]
    fn deserializes_factory_contract_config() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test/configs/factory-contract-config.yaml");

        let file_str = std::fs::read_to_string(config_path).unwrap();

        let cfg: HumanConfig = serde_yaml::from_str(&file_str).unwrap();

        println!("{:?}", cfg.networks[0].contracts[0]);

        assert!(cfg.networks[0].contracts[0].config.is_some());
        assert!(cfg.networks[0].contracts[1].config.is_some());
        assert_eq!(cfg.networks[0].contracts[1].address, None.into());
    }

    #[test]
    fn deserializes_dynamic_contract_config() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test/configs/dynamic-address-config.yaml");

        let file_str = std::fs::read_to_string(config_path).unwrap();

        let cfg: HumanConfig = serde_yaml::from_str(&file_str).unwrap();

        assert!(cfg.networks[0].contracts[0].config.is_some());
        assert!(cfg.networks[1].contracts[0].config.is_none());
    }

    #[test]
    fn deserializes_fuel_config() {
        let config_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/configs/fuel-config.yaml");

        let file_str = std::fs::read_to_string(config_path).unwrap();

        let cfg: fuel::HumanConfig = serde_yaml::from_str(&file_str).unwrap();

        let expected_cfg = fuel::HumanConfig {
            name: "Fuel indexer".to_string(),
            description: None,
            schema: None,
            ecosystem: fuel::EcosystemTag::Fuel,
            contracts: None,
            networks: vec![fuel::Network {
                id: 0,
                start_block: 0,
                end_block: None,
                contracts: vec![NetworkContract {
                    name: "OrderBook".to_string(),
                    address: "0x4a2ce054e3e94155f7092f7365b212f7f45105b74819c623744ebcc5d065c6ac"
                        .to_string()
                        .into(),
                    config: Some(fuel::ContractConfig {
                        abi_file_path: "./abis/spark-orderbook.json".to_string(),
                        handler: "./src/OrderBookHandlers.ts".to_string(),
                        events: vec![
                            fuel::EventConfig {
                                name: "OrderChangeEvent".to_string(),
                                log_id: None.into(),
                            },
                            fuel::EventConfig {
                                name: "MarketCreateEvent".to_string(),
                                log_id: None.into(),
                            },
                            fuel::EventConfig {
                                name: "TradeEvent".to_string(),
                                log_id: None.into(),
                            },
                        ],
                    }),
                }],
            }],
        };

        // deserializes fuel config
        assert_eq!(cfg, expected_cfg);
    }

    #[test]
    fn serializes_fuel_config() {
        let cfg = fuel::HumanConfig {
            name: "Fuel indexer".to_string(),
            description: None,
            schema: None,
            ecosystem: fuel::EcosystemTag::Fuel,
            contracts: None,
            networks: vec![],
        };

        assert_eq!(
            serde_yaml::to_string(&cfg).unwrap(),
            "name: Fuel indexer\necosystem: fuel\nnetworks: []\n"
        );
    }

    #[test]
    fn deserializes_event_decoder() {
        assert_eq!(
            serde_json::from_value::<EventDecoder>(json!("viem")).unwrap(),
            EventDecoder::Viem
        );
        assert_eq!(
            serde_json::from_value::<EventDecoder>(json!("hypersync-client")).unwrap(),
            EventDecoder::HypersyncClient
        );
        assert_eq!(
            serde_json::to_value(&EventDecoder::HypersyncClient).unwrap(),
            json!("hypersync-client")
        );
        assert_eq!(
            serde_json::to_value(&EventDecoder::Viem).unwrap(),
            json!("viem")
        );
    }

    #[test]
    fn deserialize_underscores_between_numbers() {
        let num = serde_json::json!(2_000_000);
        let de: i32 = serde_json::from_value(num).unwrap();
        assert_eq!(2_000_000, de);
    }

    #[test]
    fn deserialize_network_with_underscores_between_numbers() {
        let network_json = serde_json::json!({"id": 1, "start_block": 2_000, "end_block": 2_000_000, "contracts": []});
        let de: Network = serde_json::from_value(network_json).unwrap();

        assert_eq!(
            Network {
                id: 1,
                hypersync_config: None,
                rpc_config: None,
                start_block: 2_000,
                confirmed_block_threshold: None,
                end_block: Some(2_000_000),
                contracts: vec![]
            },
            de
        );
    }
}
