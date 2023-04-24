use std::path::PathBuf;
use std::{error::Error, path::Path};

pub mod entity_parsing;
pub mod event_parsing;

use serde::{Deserialize, Serialize};

use ethereum_abi::Abi;

use crate::{
    capitalization::{Capitalize, CapitalizedOptions},
    project_paths::ProjectPaths,
};

type NetworkId = i32;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct RequiredEntity {
    name: String,
    labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Event {
    name: String,
    required_entities: Option<Vec<RequiredEntity>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Network {
    id: NetworkId,
    rpc_url: String,
    start_block: i32,
    contracts: Vec<ConfigContract>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ConfigContract {
    name: String,
    // Eg for implementing a custom deserializer
    //  #[serde(deserialize_with = "abi_path_to_abi")]
    abi_file_path: String,
    handler: Option<String>,
    address: Vec<String>,
    events: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    version: String,
    description: String,
    repository: String,
    networks: Vec<Network>,
}

// fn abi_path_to_abi<'de, D>(deserializer: D) -> Result<u64, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let abi_file_path: &str = Deserialize::deserialize(deserializer)?;
//     // ... convert to abi here
// }

type StringifiedAbi = String;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct SingleContractTemplate {
    name: CapitalizedOptions,
    abi: StringifiedAbi,
    address: String,
    events: Vec<CapitalizedOptions>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ChainConfigTemplate {
    network_config: Network,
    contracts: Vec<SingleContractTemplate>,
}

pub fn deserialize_config_from_yaml(config_path: &PathBuf) -> Result<Config, Box<dyn Error>> {
    let config = std::fs::read_to_string(&config_path).map_err(|err| {
        format!(
            "Failed to resolve config path {} with Error {}",
            &config_path.to_str().unwrap_or("unknown path"),
            err.to_string()
        )
    })?;

    let deserialized_yaml: Config = serde_yaml::from_str(&config)?;
    Ok(deserialized_yaml)
}

pub fn convert_config_to_chain_configs(
    project_paths: &ProjectPaths,
) -> Result<Vec<ChainConfigTemplate>, Box<dyn Error>> {
    let config = deserialize_config_from_yaml(&project_paths.config)?;

    let mut chain_configs = Vec::new();
    for network in config.networks.iter() {
        let mut single_contracts = Vec::new();

        for contract in network.contracts.iter() {
            for contract_address in contract.address.iter() {
                let config_parent_path = &project_paths
                    .config
                    .parent()
                    .ok_or("invalid config parent directory")?;
                let abi_relative_path = Path::new(&contract.abi_file_path);
                let abi_path = config_parent_path
                    .join(abi_relative_path)
                    .canonicalize()
                    .map_err(|err| {
                        format!(
                            "Failed to resolve abi path {} with Error {}",
                            &contract.abi_file_path,
                            err.to_string()
                        )
                    })?;
                let parsed_abi: Abi =
                    event_parsing::get_abi_from_file_path(&config_parent_path.join(&abi_path))?;

                let stringified_abi = serde_json::to_string(&parsed_abi)?;
                let single_contract = SingleContractTemplate {
                    name: contract.name.to_capitalized_options(),
                    abi: stringified_abi,
                    address: contract_address.clone(),
                    events: contract
                        .events
                        .iter()
                        .map(|event| event.name.to_capitalized_options())
                        .collect(),
                };
                single_contracts.push(single_contract);
            }
        }
        let chain_config = ChainConfigTemplate {
            network_config: network.clone(),
            contracts: single_contracts,
        };
        chain_configs.push(chain_config);
    }
    Ok(chain_configs)
}

#[cfg(test)]
mod tests {
    use crate::{capitalization::Capitalize, project_paths::ProjectPaths};

    use super::ChainConfigTemplate;

    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn convert_to_chain_configs_case_1() {
        let address1 = String::from("0x2E645469f354BB4F5c8a05B3b30A929361cf77eC");
        let abi_file_path = PathBuf::from("test/abis/Contract1.json");

        let event1 = super::Event {
            name: String::from("NewGravatar"),
            required_entities: None,
        };

        let event2 = super::Event {
            name: String::from("UpdatedGravatar"),
            required_entities: None,
        };

        let contract1 = super::ConfigContract {
            handler: None,
            address: vec![address1.clone()],
            name: String::from("Contract1"),
            //needed to have relative path in order to match config1.yaml
            abi_file_path: String::from("../abis/Contract1.json"),
            events: vec![event1.clone(), event2.clone()],
        };

        let contracts = vec![contract1.clone()];
        let network1 = super::Network {
            id: 1,
            rpc_url: String::from("https://eth.com"),
            start_block: 0,
            contracts,
        };

        let config_path = PathBuf::from("test/configs/config1.yaml");
        let mut project_paths = ProjectPaths::default();
        project_paths.config = config_path;
        let chain_configs = super::convert_config_to_chain_configs(&project_paths).unwrap();
        let abi_unparsed_string =
            fs::read_to_string(abi_file_path).expect("expected json file to be at this path");
        let abi_parsed: ethereum_abi::Abi = serde_json::from_str(&abi_unparsed_string).unwrap();
        let abi_parsed_string = serde_json::to_string(&abi_parsed).unwrap();
        let single_contract1 = super::SingleContractTemplate {
            name: String::from("Contract1").to_capitalized_options(),
            abi: abi_parsed_string,
            address: address1.clone(),
            events: vec![
                event1.name.to_capitalized_options(),
                event2.name.to_capitalized_options(),
            ],
        };

        let chain_config_1 = ChainConfigTemplate {
            network_config: network1,
            contracts: vec![single_contract1],
        };

        let expected_chain_configs = vec![chain_config_1];

        assert_eq!(
            chain_configs[0].network_config,
            expected_chain_configs[0].network_config
        );
        assert_eq!(chain_configs, expected_chain_configs);
    }

    #[test]
    fn convert_to_chain_configs_case_2() {
        let address1 = String::from("0x2E645469f354BB4F5c8a05B3b30A929361cf77eC");
        let address2 = String::from("0x1E645469f354BB4F5c8a05B3b30A929361cf77eC");

        let abi_file_path = PathBuf::from("test/abis/Contract1.json");

        let event1 = super::Event {
            name: String::from("NewGravatar"),
            required_entities: None,
        };

        let event2 = super::Event {
            name: String::from("UpdatedGravatar"),
            required_entities: None,
        };

        let contract1 = super::ConfigContract {
            handler: None,
            address: vec![address1.clone()],
            name: String::from("Contract1"),
            abi_file_path: String::from("../abis/Contract1.json"),
            events: vec![event1.clone(), event2.clone()],
        };

        let contracts1 = vec![contract1.clone()];

        let network1 = super::Network {
            id: 1,
            rpc_url: String::from("https://eth.com"),
            start_block: 0,
            contracts: contracts1,
        };
        let contract2 = super::ConfigContract {
            handler: None,
            address: vec![address2.clone()],
            name: String::from("Contract1"),
            abi_file_path: String::from("../abis/Contract1.json"),
            events: vec![event1.clone(), event2.clone()],
        };

        let contracts2 = vec![contract2];

        let network2 = super::Network {
            id: 2,
            rpc_url: String::from("https://eth.com"),
            start_block: 0,
            contracts: contracts2,
        };

        let config_path = PathBuf::from("test/configs/config2.yaml");
        let mut project_paths = ProjectPaths::default();
        project_paths.config = config_path;
        let chain_configs = super::convert_config_to_chain_configs(&project_paths).unwrap();

        let events = vec![
            event1.name.to_capitalized_options(),
            event2.name.to_capitalized_options(),
        ];

        let abi_unparsed_string =
            fs::read_to_string(abi_file_path).expect("expected json file to be at this path");
        let abi_parsed: ethereum_abi::Abi = serde_json::from_str(&abi_unparsed_string).unwrap();
        let abi_parsed_string = serde_json::to_string(&abi_parsed).unwrap();
        let single_contract1 = super::SingleContractTemplate {
            name: String::from("Contract1").to_capitalized_options(),
            abi: abi_parsed_string.clone(),
            address: address1.clone(),
            events: events.clone(),
        };
        let single_contract2 = super::SingleContractTemplate {
            name: String::from("Contract1").to_capitalized_options(),
            abi: abi_parsed_string.clone(),
            address: address2.clone(),
            events,
        };

        let chain_config_1 = ChainConfigTemplate {
            network_config: network1,
            contracts: vec![single_contract1],
        };
        let chain_config_2 = ChainConfigTemplate {
            network_config: network2,
            contracts: vec![single_contract2],
        };

        let expected_chain_configs = vec![chain_config_1, chain_config_2];

        assert_eq!(chain_configs, expected_chain_configs);
    }
}
