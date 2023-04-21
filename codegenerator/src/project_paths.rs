use std::{
    error::Error,
    path::{Component, PathBuf},
};

use crate::cli_args::{
    ProjectPathsArgs, DEFAULT_CONFIG_PATH, DEFAULT_GENERATED_PATH, DEFAULT_PROJECT_ROOT_PATH,
};

#[derive(Debug, PartialEq)]
pub struct ProjectPaths {
    pub project_root: PathBuf,
    pub config: PathBuf,
    pub schema: PathBuf,
    pub generated: PathBuf,
}

impl ProjectPaths {
    pub fn new(project_paths_args: ProjectPathsArgs) -> Result<ProjectPaths, Box<dyn Error>> {
        let project_root = PathBuf::from(project_paths_args.project_root);
        let generated_relative_path = PathBuf::from(&project_paths_args.generated);
        if let Some(Component::ParentDir) = generated_relative_path.components().next() {
            return Err("Generated folder must be in project directory".into());
        }
        let generated: PathBuf = project_root.join(generated_relative_path);

        let config_relative_path = PathBuf::from(&project_paths_args.config);
        if let Some(Component::ParentDir) = config_relative_path.components().next() {
            return Err("Config path must be in project directory".into());
        }

        let config: PathBuf = project_root.join(config_relative_path);
        let schema = project_root.join("schema.graphql");

        Ok(ProjectPaths {
            project_root,
            generated,
            config,
            schema,
        })
    }

    pub fn default() -> ProjectPaths {
        let project_root = PathBuf::from(DEFAULT_PROJECT_ROOT_PATH);

        let generated: PathBuf = project_root.join(DEFAULT_GENERATED_PATH);

        let config: PathBuf = project_root.join(DEFAULT_CONFIG_PATH);
        let schema = project_root.join("schema.graphql");

        ProjectPaths {
            project_root,
            generated,
            config,
            schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectPaths;
    use crate::cli_args::ProjectPathsArgs;
    use std::path::PathBuf;
    #[test]
    fn test_project_path_default_case() {
        let project_root = String::from("./");
        let config = String::from("config.yaml");
        let generated = String::from("generated/");
        let project_paths = ProjectPaths::new(ProjectPathsArgs {
            project_root,
            config,
            generated,
        })
        .unwrap();

        let expected_project_paths = ProjectPaths {
            project_root: PathBuf::from("./"),
            config: PathBuf::from("./config.yaml"),
            schema: PathBuf::from("./schema.graphql"),
            generated: PathBuf::from("./generated"),
        };
        assert_eq!(project_paths, expected_project_paths)
    }
    #[test]
    fn test_project_path_alternative_case() {
        let project_root = String::from("my_dir/my_project");
        let config = String::from("custom_config.yaml");
        let generated = String::from("custom_gen/my_project_generated");
        let project_paths = ProjectPaths::new(ProjectPathsArgs {
            project_root,
            config,
            generated,
        })
        .unwrap();

        let expected_project_paths = ProjectPaths {
            project_root: PathBuf::from("my_dir/my_project/"),
            config: PathBuf::from("my_dir/my_project/custom_config.yaml"),
            schema: PathBuf::from("my_dir/my_project/schema.graphql"),
            generated: PathBuf::from("my_dir/my_project/custom_gen/my_project_generated"),
        };
        assert_eq!(project_paths, expected_project_paths)
    }
    #[test]
    fn test_project_path_relative_case() {
        let project_root = String::from("../my_dir/my_project");
        let config = String::from("custom_config.yaml");
        let generated = String::from("custom_gen/my_project_generated");
        let project_paths = ProjectPaths::new(ProjectPathsArgs {
            project_root,
            config,
            generated,
        })
        .unwrap();

        let expected_project_paths = ProjectPaths {
            project_root: PathBuf::from("../my_dir/my_project/"),
            config: PathBuf::from("../my_dir/my_project/custom_config.yaml"),
            schema: PathBuf::from("../my_dir/my_project/schema.graphql"),
            generated: PathBuf::from("../my_dir/my_project/custom_gen/my_project_generated"),
        };
        assert_eq!(project_paths, expected_project_paths)
    }

    #[test]
    #[should_panic]
    fn test_project_path_panics_when_generated_is_outside_of_root() {
        let project_root = String::from("./");
        let config = String::from("config.yaml");
        let generated = String::from("../generated/");
        ProjectPaths::new(ProjectPathsArgs {
            project_root,
            config,
            generated,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn test_project_path_panics_when_config_is_outside_of_root() {
        let project_root = String::from("./");
        let config = String::from("../config.yaml");
        let generated = String::from("generated/");
        ProjectPaths::new(ProjectPathsArgs {
            project_root,
            config,
            generated,
        })
        .unwrap();
    }
}
