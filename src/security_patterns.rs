use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Python,
    JavaScript,
    Rust,
    TypeScript,
    Java,
    Go,
    Ruby,
    C,
    Cpp,
    Terraform,
    CloudFormation,
    Kubernetes,
    Yaml,
    Bash,
    Shell,
    Php,
    Other,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "py" => Language::Python,
            "js" => Language::JavaScript,
            "rs" => Language::Rust,
            "ts" => Language::TypeScript,
            "java" => Language::Java,
            "go" => Language::Go,
            "rb" => Language::Ruby,
            "c" | "h" => Language::C,
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Language::Cpp,
            "tf" | "hcl" => Language::Terraform,
            "yml" | "yaml" => Language::Yaml,
            "sh" | "bash" => Language::Bash,
            "php" | "php3" | "php4" | "php5" | "phtml" => Language::Php,
            _ => Language::Other,
        }
    }

    pub fn from_filename(filename: &str) -> Self {
        // Extract extension and use existing logic
        if let Some(ext) = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
        {
            Self::from_extension(ext)
        } else {
            Language::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    Principal, // Who: user input (Programming) | AWS account/role (IaC)
    Action,    // What: operations/methods (Programming) | API actions (IaC)
    Resource,  // Where: files/databases (Programming) | AWS resources (IaC)
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternConfig {
    pub pattern: String,
    pub description: String,
    pub attack_vector: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguagePatterns {
    pub principals: Option<Vec<PatternConfig>>, // Who (sources of authority/input)
    pub actions: Option<Vec<PatternConfig>>,    // What (operations/permissions)
    pub resources: Option<Vec<PatternConfig>>,  // Where (targets/sinks)
}

pub struct SecurityRiskPatterns {
    principal_patterns: Vec<Regex>, // Who patterns
    action_patterns: Vec<Regex>,    // What patterns
    resource_patterns: Vec<Regex>,  // Where patterns
    pattern_type_map: HashMap<String, PatternType>,
    attack_vector_map: HashMap<String, Vec<String>>,
}

impl SecurityRiskPatterns {
    pub fn new(language: Language) -> Self {
        Self::new_with_root(language, None)
    }

    pub fn new_with_root(language: Language, root_dir: Option<&Path>) -> Self {
        let pattern_map = Self::load_patterns(root_dir);
        let lang_patterns = pattern_map
            .get(&language)
            .or_else(|| pattern_map.get(&Language::Other))
            .unwrap_or(&LanguagePatterns {
                principals: None,
                actions: None,
                resources: None,
            });

        let mut principal_patterns = Vec::new();
        let mut action_patterns = Vec::new();
        let mut resource_patterns = Vec::new();
        let mut pattern_type_map = HashMap::new();
        let mut attack_vector_map = HashMap::new();

        if let Some(principals) = &lang_patterns.principals {
            for config in principals {
                let regex = Regex::new(&config.pattern).unwrap();
                pattern_type_map.insert(config.pattern.clone(), PatternType::Principal);
                if !config.attack_vector.is_empty() {
                    attack_vector_map.insert(config.pattern.clone(), config.attack_vector.clone());
                }
                principal_patterns.push(regex);
            }
        }

        if let Some(actions) = &lang_patterns.actions {
            for config in actions {
                let regex = Regex::new(&config.pattern).unwrap();
                pattern_type_map.insert(config.pattern.clone(), PatternType::Action);
                if !config.attack_vector.is_empty() {
                    attack_vector_map.insert(config.pattern.clone(), config.attack_vector.clone());
                }
                action_patterns.push(regex);
            }
        }

        if let Some(resources) = &lang_patterns.resources {
            for config in resources {
                let regex = Regex::new(&config.pattern).unwrap();
                pattern_type_map.insert(config.pattern.clone(), PatternType::Resource);
                if !config.attack_vector.is_empty() {
                    attack_vector_map.insert(config.pattern.clone(), config.attack_vector.clone());
                }
                resource_patterns.push(regex);
            }
        }

        Self {
            principal_patterns,
            action_patterns,
            resource_patterns,
            pattern_type_map,
            attack_vector_map,
        }
    }

    pub fn matches(&self, content: &str) -> bool {
        self.principal_patterns
            .iter()
            .any(|pattern| pattern.is_match(content))
            || self
                .action_patterns
                .iter()
                .any(|pattern| pattern.is_match(content))
            || self
                .resource_patterns
                .iter()
                .any(|pattern| pattern.is_match(content))
    }

    pub fn get_pattern_type(&self, content: &str) -> Option<PatternType> {
        for (pattern_str, pattern_type) in &self.pattern_type_map {
            let regex = Regex::new(pattern_str).ok()?;
            if regex.is_match(content) {
                return Some(pattern_type.clone());
            }
        }
        None
    }

    pub fn get_attack_vectors(&self, content: &str) -> Vec<String> {
        for (pattern_str, attack_vectors) in &self.attack_vector_map {
            if let Ok(regex) = Regex::new(pattern_str) {
                if regex.is_match(content) {
                    return attack_vectors.clone();
                }
            }
        }
        Vec::new()
    }

    fn load_patterns(root_dir: Option<&Path>) -> HashMap<Language, LanguagePatterns> {
        use Language::*;

        let mut map = HashMap::new();

        // Load patterns from individual language files
        let languages = [
            (Python, include_str!("patterns/python.yml")),
            (JavaScript, include_str!("patterns/javascript.yml")),
            (Rust, include_str!("patterns/rust.yml")),
            (TypeScript, include_str!("patterns/typescript.yml")),
            (Java, include_str!("patterns/java.yml")),
            (Go, include_str!("patterns/go.yml")),
            (Ruby, include_str!("patterns/ruby.yml")),
            (C, include_str!("patterns/c.yml")),
            (Cpp, include_str!("patterns/cpp.yml")),
            (Terraform, include_str!("patterns/terraform.yml")),
            (Kubernetes, include_str!("patterns/kubernetes.yml")),
            (Yaml, include_str!("patterns/yaml.yml")),
            (Bash, include_str!("patterns/bash.yml")),
            (Php, include_str!("patterns/php.yml")),
        ];

        for (lang, content) in languages {
            match serde_yaml::from_str::<LanguagePatterns>(content) {
                Ok(patterns) => {
                    map.insert(lang, patterns);
                }
                Err(e) => {
                    eprintln!("Failed to parse patterns for {:?}: {}", lang, e);
                }
            }
        }

        // Load custom patterns from vuln-patterns.yml if it exists
        Self::load_custom_patterns(&mut map, root_dir);

        map
    }

    fn load_custom_patterns(
        map: &mut HashMap<Language, LanguagePatterns>,
        root_dir: Option<&Path>,
    ) {
        let vuln_patterns_path = if let Some(root) = root_dir {
            root.join("vuln-patterns.yml")
        } else {
            Path::new("vuln-patterns.yml").to_path_buf()
        };

        if vuln_patterns_path.exists() {
            match std::fs::read_to_string(&vuln_patterns_path) {
                Ok(content) => {
                    // Parse the entire file as a map of language names to patterns
                    match serde_yaml::from_str::<HashMap<String, LanguagePatterns>>(&content) {
                        Ok(custom_patterns) => {
                            for (lang_name, patterns) in custom_patterns {
                                // Convert language name to Language enum
                                let language = match lang_name.as_str() {
                                    "Python" => Language::Python,
                                    "JavaScript" => Language::JavaScript,
                                    "TypeScript" => Language::TypeScript,
                                    "Rust" => Language::Rust,
                                    "Java" => Language::Java,
                                    "Go" => Language::Go,
                                    "Ruby" => Language::Ruby,
                                    "C" => Language::C,
                                    "Cpp" => Language::Cpp,
                                    "Terraform" => Language::Terraform,
                                    "CloudFormation" => Language::CloudFormation,
                                    "Kubernetes" => Language::Kubernetes,
                                    "YAML" => Language::Yaml,
                                    "Bash" => Language::Bash,
                                    "Shell" => Language::Shell,
                                    "Php" | "PHP" => Language::Php,
                                    _ => continue,
                                };

                                // Merge custom patterns with existing patterns
                                match map.get_mut(&language) {
                                    Some(existing) => {
                                        // Merge principals
                                        if let Some(custom_principals) = patterns.principals {
                                            match &mut existing.principals {
                                                Some(principals) => {
                                                    principals.extend(custom_principals)
                                                }
                                                None => {
                                                    existing.principals = Some(custom_principals)
                                                }
                                            }
                                        }
                                        // Merge actions
                                        if let Some(custom_actions) = patterns.actions {
                                            match &mut existing.actions {
                                                Some(actions) => actions.extend(custom_actions),
                                                None => existing.actions = Some(custom_actions),
                                            }
                                        }
                                        // Merge resources
                                        if let Some(custom_resources) = patterns.resources {
                                            match &mut existing.resources {
                                                Some(resources) => {
                                                    resources.extend(custom_resources)
                                                }
                                                None => existing.resources = Some(custom_resources),
                                            }
                                        }
                                    }
                                    None => {
                                        // Insert new language patterns
                                        map.insert(language, patterns);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse vuln-patterns.yml: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read vuln-patterns.yml: {}", e);
                }
            }
        }
    }
}
