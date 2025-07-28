use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::client_common::Prompt;
use crate::error::CodexErr;
use crate::error::Result;
use crate::models::ResponseItem;
use serde::Deserialize;
use serde::Serialize;

/// YAML prompt message structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YamlMessage {
    pub role: String,
    pub content: String,
}

/// YAML prompt file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YamlPrompt {
    pub messages: Vec<YamlMessage>,
    pub model: Option<String>,
}

impl YamlPrompt {
    /// Load a YAML prompt from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path).map_err(|e| {
            CodexErr::IO(format!(
                "Failed to read YAML prompt file {:?}: {}",
                path.as_ref(),
                e
            ))
        })?;

        let yaml_prompt: YamlPrompt = serde_yaml::from_str(&content)
            .map_err(|e| CodexErr::IO(format!("Failed to parse YAML prompt: {}", e)))?;

        Ok(yaml_prompt)
    }

    /// Convert YAML prompt to internal Prompt structure with template variable substitution
    pub fn to_prompt(&self, template_vars: &HashMap<String, String>) -> Result<Prompt> {
        let mut response_items = Vec::new();

        for msg in &self.messages {
            let content = substitute_template_vars(&msg.content, template_vars);

            let response_item = match msg.role.as_str() {
                "system" => ResponseItem::system_message(content),
                "user" => ResponseItem::user_message(content),
                "assistant" => ResponseItem::assistant_message(content),
                _ => {
                    return Err(CodexErr::IO(format!(
                        "Unsupported message role: {}",
                        msg.role
                    )));
                }
            };

            response_items.push(response_item);
        }

        let mut prompt = Prompt::default();
        prompt.input = response_items;

        // If the YAML contains a system message, use it as base_instructions_override
        if let Some(system_msg) = self.messages.iter().find(|m| m.role == "system") {
            let system_content = substitute_template_vars(&system_msg.content, template_vars);
            prompt.base_instructions_override = Some(system_content);
            // Remove system message from input since it's now in base_instructions_override
            prompt
                .input
                .retain(|item| !matches!(item, ResponseItem::SystemMessage { .. }));
        }

        Ok(prompt)
    }

    /// Get the model specified in the YAML prompt
    pub fn get_model(&self) -> Option<&str> {
        self.model.as_deref()
    }
}

/// Substitute template variables in content string
fn substitute_template_vars(content: &str, vars: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    for (key, value) in vars {
        let template = format!("{{{{{}}}}}", key);
        result = result.replace(&template, value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_template_substitution() {
        let mut vars = HashMap::new();
        vars.insert("input".to_string(), "test input".to_string());
        vars.insert("name".to_string(), "Alice".to_string());

        let content = "Hello {{name}}, process this: {{input}}";
        let result = substitute_template_vars(content, &vars);

        assert_eq!(result, "Hello Alice, process this: test input");
    }

    #[test]
    fn test_yaml_prompt_parsing() {
        let yaml_content = r#"
messages:
  - role: system
    content: "You are a helpful assistant"
  - role: user
    content: "Process this: {{input}}"
model: "gpt-4"
"#;

        let prompt: YamlPrompt = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(prompt.messages.len(), 2);
        assert_eq!(prompt.messages[0].role, "system");
        assert_eq!(prompt.model, Some("gpt-4".to_string()));
    }
}
