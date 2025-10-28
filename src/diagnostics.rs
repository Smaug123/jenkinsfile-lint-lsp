use regex::Regex;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

const SUCCESS_MESSAGE: &str = "Jenkinsfile successfully validated.";

/// Parse Jenkins validation response and convert to LSP diagnostics
///
/// Jenkins returns errors in the format:
/// "WorkflowScript: 46: unexpected token: } @ line 46, column 1."
///
/// Multiple errors may be present across multiple lines of output.
pub fn parse_jenkins_response(response: &str) -> Vec<Diagnostic> {
    // Check for success message first
    if response.contains(SUCCESS_MESSAGE) {
        return Vec::new();
    }

    // Regex pattern to match Jenkins error format
    // WorkflowScript: <num>: <message> @ line <line>, column <col>.
    let re = Regex::new(r"WorkflowScript:\s+\d+:\s+(.+?)\s+@\s+line\s+(\d+),\s+column\s+(\d+)\.")
        .expect("Invalid regex pattern");

    let mut diagnostics = Vec::new();

    for line in response.lines() {
        if let Some(captures) = re.captures(line) {
            // Extract message, line number, and column number
            let message = captures.get(1).map(|m| m.as_str()).unwrap_or("Unknown error");
            let line_str = captures.get(2).map(|m| m.as_str()).unwrap_or("0");
            let col_str = captures.get(3).map(|m| m.as_str()).unwrap_or("0");

            // Parse line and column numbers
            if let (Ok(line_num), Ok(col_num)) = (line_str.parse::<u32>(), col_str.parse::<u32>()) {
                // LSP uses 0-indexed line and column numbers
                let line = line_num.saturating_sub(1);
                let col = col_num.saturating_sub(1);

                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: col,
                        },
                        end: Position {
                            line,
                            character: col,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("jenkinsfile-ls".to_string()),
                    message: message.to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics.push(diagnostic);
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success_message() {
        let response = "Jenkinsfile successfully validated.";
        let diagnostics = parse_jenkins_response(response);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_single_error() {
        let response = "WorkflowScript: 46: unexpected token: } @ line 46, column 1.";
        let diagnostics = parse_jenkins_response(response);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "unexpected token: }");
        assert_eq!(diagnostics[0].range.start.line, 45); // 0-indexed
        assert_eq!(diagnostics[0].range.start.character, 0); // 0-indexed
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostics[0].source, Some("jenkinsfile-ls".to_string()));
    }

    #[test]
    fn test_parse_multiple_errors() {
        let response = r#"
WorkflowScript: 10: Unexpected input @ line 10, column 5.
Some other output line
WorkflowScript: 20: Missing closing brace @ line 20, column 3.
        "#;

        let diagnostics = parse_jenkins_response(response);

        assert_eq!(diagnostics.len(), 2);

        // First error
        assert_eq!(diagnostics[0].message, "Unexpected input");
        assert_eq!(diagnostics[0].range.start.line, 9);
        assert_eq!(diagnostics[0].range.start.character, 4);

        // Second error
        assert_eq!(diagnostics[1].message, "Missing closing brace");
        assert_eq!(diagnostics[1].range.start.line, 19);
        assert_eq!(diagnostics[1].range.start.character, 2);
    }

    #[test]
    fn test_parse_no_errors() {
        let response = "Some random output\nwithout any errors";
        let diagnostics = parse_jenkins_response(response);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_complex_message() {
        let response =
            "WorkflowScript: 15: expecting '}', found 'stage' @ line 15, column 10.";
        let diagnostics = parse_jenkins_response(response);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "expecting '}', found 'stage'");
        assert_eq!(diagnostics[0].range.start.line, 14);
        assert_eq!(diagnostics[0].range.start.character, 9);
    }
}
