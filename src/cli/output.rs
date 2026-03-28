use crate::daemon::protocol::Response;

/// Format a response as JSON for agent consumption.
pub fn format_json(response: &Response) -> String {
    serde_json::to_string_pretty(response).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"error":"Failed to serialize response: {}"}}"#,
            e
        )
    })
}

/// Format a response as plain text for human debugging.
pub fn format_plain(response: &Response) -> String {
    if !response.success {
        return format!(
            "Error: {}",
            response.error.as_deref().unwrap_or("Unknown error")
        );
    }

    match &response.data {
        Some(crate::daemon::protocol::ResponseData::Session(info)) => {
            format!(
                "Session: {}\nState: {}\nCommand: {}\nPID: {}\nLines: {}\nTranscript: {}",
                info.name,
                info.state,
                info.command.join(" "),
                info.pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                info.output_lines,
                info.transcript_path,
            )
        }
        Some(crate::daemon::protocol::ResponseData::Output(data)) => {
            format!(
                "Session: {} (cursor: {}, {} lines)\n---\n{}\n---\nOutput file: {}",
                data.session, data.cursor, data.lines, data.text, data.output_file
            )
        }
        Some(crate::daemon::protocol::ResponseData::SessionList(sessions)) => {
            if sessions.is_empty() {
                return "No active sessions".to_string();
            }
            let mut lines = vec!["NAME\tSTATE\tCOMMAND\tLINES".to_string()];
            for s in sessions {
                lines.push(format!(
                    "{}\t{}\t{}\t{}",
                    s.name,
                    s.state,
                    s.command.join(" "),
                    s.output_lines
                ));
            }
            lines.join("\n")
        }
        Some(crate::daemon::protocol::ResponseData::WaitResult(result)) => {
            if result.matched {
                format!(
                    "Pattern '{}' matched: {}",
                    result.pattern,
                    result.match_text.as_deref().unwrap_or("(match)")
                )
            } else {
                format!("Pattern '{}' did not match (timed out)", result.pattern)
            }
        }
        Some(crate::daemon::protocol::ResponseData::Ok(data)) => {
            format!("{}: {}", data.session, data.message)
        }
        None => "OK".to_string(),
    }
}
