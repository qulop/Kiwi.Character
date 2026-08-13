//! Embedded prompt assets and their small, deliberately strict template renderer.
//!
//! `include_str!` copies these files into the application binary at compile time.
//! Keeping their prose and XML layout outside command code makes them easy to
//! review and tune without mixing prompt writing with application logic.

const SYSTEM_PROMPT_TEMPLATE: &str = include_str!("../prompts/system_prompt.xml");
const XML_INTERPRETATION: &str = include_str!("../prompts/xml_interpretation.txt");
const ROLEPLAY_RULES: &str = include_str!("../prompts/roleplay_rules.txt");
const CHARACTER_TEMPLATE: &str = include_str!("../prompts/character.xml");
const PERSONA_TEMPLATE: &str = include_str!("../prompts/user_persona.xml");
const ADDITIONAL_INSTRUCTIONS_TEMPLATE: &str =
    include_str!("../prompts/additional_user_instructions.xml");
const OPENING_CONTEXT_TEMPLATE: &str = include_str!("../prompts/opening_context.xml");

/// Render the complete application-owned system prompt from embedded assets.
pub fn system_prompt(
    character_block: &str,
    persona_block: &str,
    additional_user_instructions: &str,
    opening_context: &str,
) -> String {
    return render(
        SYSTEM_PROMPT_TEMPLATE,
        &[
            ("xml_interpretation", XML_INTERPRETATION.trim_end()),
            ("roleplay_rules", ROLEPLAY_RULES.trim_end()),
            ("character_block", character_block),
            ("persona_block", persona_block),
            ("additional_user_instructions", additional_user_instructions),
            ("opening_context", opening_context),
        ],
    );
}

/// Render the seeded character greeting as private context for the first reply.
/// The value must already be XML-escaped.
pub fn opening_context(message: &str) -> String {
    return render(OPENING_CONTEXT_TEMPLATE, &[("message", message)]);
}

/// Render a character reference block. Values must already be XML-escaped.
pub fn character_block(
    name: &str,
    short_info: &str,
    appearance: &str,
    description: &str,
) -> String {
    return render(
        CHARACTER_TEMPLATE,
        &[
            ("name", name),
            ("short_info", short_info),
            ("appearance", appearance),
            ("description", description),
        ],
    );
}

/// Render a selected user-persona reference block. Values must be XML-escaped.
pub fn persona_block(name: &str, description: &str) -> String {
    return render(
        PERSONA_TEMPLATE,
        &[("name", name), ("description", description)],
    );
}

/// Render optional user-authored global guidance. The value must be XML-escaped.
pub fn additional_user_instructions(instructions: &str) -> String {
    return render(
        ADDITIONAL_INSTRUCTIONS_TEMPLATE,
        &[("instructions", instructions)],
    );
}

/// Replace only `{{known_placeholder}}` tokens in a template. A one-pass parser
/// prevents user text containing a placeholder from being substituted again.
fn render(template: &str, values: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(open) = remaining.find("{{") {
        result.push_str(&remaining[..open]);
        let after_open = &remaining[open + 2..];
        let close = after_open
            .find("}}")
            .expect("embedded prompt template has an unclosed placeholder");
        let key = &after_open[..close];
        let value = values
            .iter()
            .find_map(|(name, value)| (*name == key).then_some(*value))
            .unwrap_or_else(|| {
                panic!("embedded prompt template has unknown placeholder '{key}'")
            });
        result.push_str(value);
        remaining = &after_open[close + 2..];
    }

    result.push_str(remaining);
    return result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_does_not_reprocess_placeholder_text_in_values() {
        let rendered = render("<x>{{value}}</x>", &[("value", "{{value}}")]);
        assert_eq!(rendered, "<x>{{value}}</x>");
    }
}
