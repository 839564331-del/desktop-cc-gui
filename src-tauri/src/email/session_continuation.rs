use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use imap::ConnectionMode;
use mailparse::{MailHeaderMap, ParsedMail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::storage::{read_json_file, write_json_file};
use crate::types::{EmailInboundSecurity, EmailInboundSettings, EmailSenderProvider};

type HmacSha256 = Hmac<Sha256>;

pub(crate) const EMAIL_SESSION_LEDGER_FILE_NAME: &str = "email-session-ledger.json";
pub(crate) const EMAIL_REPLY_DELIMITER: &str = "--- Reply above this line ---";
const MOSS_CONTEXT_START: &str = "--- MOSS CONTEXT ---";
const MOSS_CONTEXT_END: &str = "--- END MOSS CONTEXT ---";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EmailMailCommandAction {
    Next,
    Change,
    Pause,
    Stop,
    Status,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OutgoingMailStatus {
    NonActionable,
    Actionable,
    Consumed,
    Superseded,
    Expired,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InboundCommandStatus {
    Accepted,
    Queued,
    Running,
    Done,
    NeedsConfirmation,
    Duplicate,
    Expired,
    Rejected,
    Ignored,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MailSessionState {
    Enabled,
    Paused,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateEmailInboundSettingsRequest {
    pub(crate) settings: EmailInboundSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailInboundSettingsView {
    pub(crate) settings: EmailInboundSettings,
    pub(crate) read_only_effective: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailInboundListenerStatus {
    pub(crate) enabled: bool,
    pub(crate) read_only: bool,
    pub(crate) connection_state: String,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) next_check_at: Option<String>,
    pub(crate) accepted_count: usize,
    pub(crate) queued_count: usize,
    pub(crate) needs_confirmation_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) ignored_count: usize,
    pub(crate) polling_interval_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailMailSessionRow {
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
    pub(crate) workspace_name: Option<String>,
    pub(crate) thread_name: Option<String>,
    pub(crate) state: MailSessionState,
    pub(crate) last_event_at: Option<String>,
    pub(crate) latest_action: Option<EmailMailCommandAction>,
    pub(crate) latest_status: Option<InboundCommandStatus>,
    pub(crate) latest_reject_reason: Option<String>,
    pub(crate) outbound_count: usize,
    pub(crate) inbound_count: usize,
    pub(crate) queued_count: usize,
    pub(crate) needs_confirmation_count: usize,
    pub(crate) latest_summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailMailTimelineEvent {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) direction: String,
    pub(crate) action: Option<EmailMailCommandAction>,
    pub(crate) status: String,
    pub(crate) subject: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) reject_reason: Option<String>,
    pub(crate) occurred_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailMailSessionList {
    pub(crate) listener: EmailInboundListenerStatus,
    pub(crate) sessions: Vec<EmailMailSessionRow>,
    pub(crate) timeline: Vec<EmailMailTimelineEvent>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmailInboundMessage {
    #[serde(default)]
    pub(crate) uid: Option<String>,
    #[serde(default, rename = "messageId")]
    pub(crate) message_id: Option<String>,
    pub(crate) from: String,
    pub(crate) subject: String,
    #[serde(default, rename = "textBody")]
    pub(crate) text_body: String,
    #[serde(default, rename = "inReplyTo")]
    pub(crate) in_reply_to: Option<String>,
    #[serde(default)]
    pub(crate) references: Vec<String>,
    #[serde(default)]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(default, rename = "autoSubmitted")]
    pub(crate) auto_submitted: Option<String>,
    #[serde(default, rename = "receivedAt")]
    pub(crate) received_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckEmailInboxRequest {
    #[serde(default)]
    pub(crate) messages: Vec<EmailInboundMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckEmailInboxResult {
    pub(crate) checked_at: String,
    pub(crate) read_only: bool,
    pub(crate) scanned_count: usize,
    pub(crate) accepted_count: usize,
    pub(crate) queued_count: usize,
    pub(crate) needs_confirmation_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) ignored_count: usize,
    pub(crate) duplicate_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MutateMailSessionRequest {
    pub(crate) session_id: String,
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) command_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaimMailCommandResult {
    pub(crate) command: Option<InboundMailCommand>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompleteMailCommandRequest {
    pub(crate) command_id: String,
    pub(crate) status: InboundCommandStatus,
    #[serde(default, rename = "rejectReason")]
    pub(crate) reject_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutgoingContinuationRegistration {
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    pub(crate) workspace_name: Option<String>,
    pub(crate) thread_id: String,
    pub(crate) thread_name: Option<String>,
    pub(crate) turn_id: String,
    pub(crate) subject: String,
    pub(crate) summary: String,
    pub(crate) next_recommendations: Vec<String>,
    pub(crate) actionable: bool,
    pub(crate) action_window_hours: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedOutgoingContinuation {
    pub(crate) record: OutgoingMailRecord,
    pub(crate) subject: String,
    pub(crate) text_body: String,
    pub(crate) message_id: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutgoingMailRecord {
    pub(crate) id: String,
    #[serde(default, rename = "messageId")]
    pub(crate) message_id: Option<String>,
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    #[serde(default)]
    pub(crate) workspace_name: Option<String>,
    pub(crate) thread_id: String,
    #[serde(default)]
    pub(crate) thread_name: Option<String>,
    pub(crate) turn_id: String,
    #[serde(default, rename = "replyTokenHash")]
    pub(crate) reply_token_hash: Option<String>,
    #[serde(default, rename = "expiresAt")]
    pub(crate) expires_at: Option<String>,
    pub(crate) status: OutgoingMailStatus,
    #[serde(default, rename = "sentAt")]
    pub(crate) sent_at: String,
    #[serde(default, rename = "subjectTag")]
    pub(crate) subject_tag: Option<String>,
    pub(crate) subject: String,
    pub(crate) summary: String,
    #[serde(default, rename = "nextRecommendations")]
    pub(crate) next_recommendations: Vec<String>,
    #[serde(default)]
    pub(crate) signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InboundMailCommand {
    pub(crate) id: String,
    #[serde(rename = "mailMessageId")]
    pub(crate) mail_message_id: String,
    #[serde(default, rename = "inReplyTo")]
    pub(crate) in_reply_to: Option<String>,
    #[serde(rename = "linkedOutgoingMailId")]
    pub(crate) linked_outgoing_mail_id: String,
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
    #[serde(rename = "replyTokenHash")]
    pub(crate) reply_token_hash: String,
    #[serde(rename = "fromHash")]
    pub(crate) from_hash: String,
    #[serde(default, rename = "fromDisplay")]
    pub(crate) from_display: Option<String>,
    #[serde(rename = "receivedAt")]
    pub(crate) received_at: String,
    pub(crate) action: EmailMailCommandAction,
    #[serde(default)]
    pub(crate) detail: Option<String>,
    #[serde(rename = "bodyHash")]
    pub(crate) body_hash: String,
    pub(crate) status: InboundCommandStatus,
    #[serde(default, rename = "rejectReason")]
    pub(crate) reject_reason: Option<String>,
    #[serde(default, rename = "subjectTag")]
    pub(crate) subject_tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct MailSessionControl {
    session_id: String,
    state: MailSessionState,
    #[serde(default, rename = "mailDrivenEnabled")]
    mail_driven_enabled: bool,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
struct EmailMailboxCursor {
    #[serde(default, rename = "lastCheckedAt")]
    last_checked_at: Option<String>,
    #[serde(default, rename = "lastSeenUid")]
    last_seen_uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
struct EmailSessionLedger {
    #[serde(default)]
    outgoing: Vec<OutgoingMailRecord>,
    #[serde(default)]
    commands: Vec<InboundMailCommand>,
    #[serde(default)]
    sessions: Vec<MailSessionControl>,
    #[serde(default)]
    cursor: EmailMailboxCursor,
}

pub(crate) trait MailboxReader {
    fn read_since(&self, cursor: &Option<String>) -> Result<Vec<EmailInboundMessage>, String>;
    fn is_read_only(&self) -> bool;
}

pub(crate) struct MemoryMailboxReader {
    messages: Vec<EmailInboundMessage>,
    read_only: bool,
}

impl MemoryMailboxReader {
    pub(crate) fn new(messages: Vec<EmailInboundMessage>) -> Self {
        Self {
            messages,
            read_only: true,
        }
    }
}

impl MailboxReader for MemoryMailboxReader {
    fn read_since(&self, _cursor: &Option<String>) -> Result<Vec<EmailInboundMessage>, String> {
        Ok(self.messages.clone())
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }
}

pub(crate) struct ImapMailboxReader {
    settings: EmailInboundSettings,
    secret: String,
    max_messages_per_check: usize,
}

impl ImapMailboxReader {
    pub(crate) fn new(settings: EmailInboundSettings, secret: String) -> Self {
        Self {
            settings,
            secret,
            max_messages_per_check: 50,
        }
    }
}

impl MailboxReader for ImapMailboxReader {
    fn read_since(&self, cursor: &Option<String>) -> Result<Vec<EmailInboundMessage>, String> {
        read_imap_messages_since(
            &self.settings,
            &self.secret,
            cursor,
            self.max_messages_per_check,
        )
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedReplyCommand {
    Command {
        action: EmailMailCommandAction,
        detail: Option<String>,
    },
    NeedsConfirmation(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedMossContext {
    session_id: Option<String>,
    workspace_id: Option<String>,
    thread_id: Option<String>,
    turn_id: Option<String>,
    reply_token: Option<String>,
    expires_at: Option<String>,
    signature: Option<String>,
}

pub(crate) fn ledger_path_from_settings_path(settings_path: &Path) -> PathBuf {
    settings_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(EMAIL_SESSION_LEDGER_FILE_NAME)
}

pub(crate) fn normalize_inbound_settings(
    settings: &EmailInboundSettings,
    fallback_recipient: &str,
) -> EmailInboundSettings {
    let mut next = settings.clone();
    next.imap_host = next.imap_host.trim().to_string();
    next.username = next.username.trim().to_string();
    next.mailbox_folder = next.mailbox_folder.trim().to_string();
    if next.mailbox_folder.is_empty() {
        next.mailbox_folder = "INBOX".to_string();
    }
    next.allowed_senders = normalize_allowed_senders(&next.allowed_senders, fallback_recipient);
    next.poll_interval_seconds = next.poll_interval_seconds.clamp(10, 3600);
    next.action_window_hours = next.action_window_hours.clamp(1, 168);
    next.read_only_mode = true;
    match next.provider {
        EmailSenderProvider::Mail126 => {
            next.imap_host = "imap.126.com".to_string();
            next.imap_port = 993;
            next.security = EmailInboundSecurity::SslTls;
        }
        EmailSenderProvider::Mail163 => {
            next.imap_host = "imap.163.com".to_string();
            next.imap_port = 993;
            next.security = EmailInboundSecurity::SslTls;
        }
        EmailSenderProvider::Qq => {
            next.imap_host = "imap.qq.com".to_string();
            next.imap_port = 993;
            next.security = EmailInboundSecurity::SslTls;
        }
        EmailSenderProvider::Custom => {
            if next.imap_port == 0 {
                next.imap_port = 993;
            }
        }
    }
    next
}

fn read_imap_messages_since(
    settings: &EmailInboundSettings,
    secret: &str,
    cursor: &Option<String>,
    max_messages_per_check: usize,
) -> Result<Vec<EmailInboundMessage>, String> {
    validate_imap_settings(settings, secret)?;
    let client = imap::ClientBuilder::new(settings.imap_host.as_str(), settings.imap_port)
        .mode(connection_mode_for_security(&settings.security))
        .connect()
        .map_err(|error| format!("email inbox check failed: IMAP connection failed: {error}"))?;
    let mut session = client
        .login(settings.username.as_str(), secret)
        .map_err(|(error, _)| {
            format!("email inbox check failed: IMAP authentication failed: {error}")
        })?;
    send_provider_client_identity(&mut session, settings)?;
    session
        .select(settings.mailbox_folder.as_str())
        .map_err(|error| format!("email inbox check failed: IMAP select failed: {error}"))?;

    let mut uids = session
        .uid_search("ALL")
        .map_err(|error| format!("email inbox check failed: IMAP search failed: {error}"))?
        .into_iter()
        .collect::<Vec<_>>();
    uids.sort_unstable();
    let cursor_uid = cursor
        .as_deref()
        .and_then(|value| value.trim().parse::<u32>().ok());
    let mut candidate_uids = uids
        .into_iter()
        .filter(|uid| cursor_uid.map(|cursor| *uid > cursor).unwrap_or(true))
        .collect::<Vec<_>>();
    if cursor_uid.is_none() && candidate_uids.len() > max_messages_per_check {
        candidate_uids = candidate_uids
            .into_iter()
            .rev()
            .take(max_messages_per_check)
            .collect::<Vec<_>>();
        candidate_uids.reverse();
    } else if candidate_uids.len() > max_messages_per_check {
        candidate_uids.truncate(max_messages_per_check);
    }
    if candidate_uids.is_empty() {
        if let Err(error) = session.logout() {
            log::warn!("email inbox check IMAP logout failed after empty fetch: {error}");
        }
        return Ok(Vec::new());
    }

    let uid_set = format_uid_set(&candidate_uids);
    let fetches = session
        .uid_fetch(uid_set, "(UID INTERNALDATE BODY.PEEK[])")
        .map_err(|error| format!("email inbox check failed: IMAP fetch failed: {error}"))?;
    let mut messages = Vec::new();
    for fetch in fetches.iter() {
        let Some(body) = fetch.body() else {
            continue;
        };
        let received_at = fetch
            .internal_date()
            .map(|date| date.with_timezone(&Utc).to_rfc3339());
        let parsed = parse_raw_email_message(fetch.uid, body, received_at)?;
        messages.push(parsed);
    }
    if let Err(error) = session.logout() {
        log::warn!("email inbox check IMAP logout failed: {error}");
    }
    Ok(messages)
}

fn validate_imap_settings(settings: &EmailInboundSettings, secret: &str) -> Result<(), String> {
    if settings.imap_host.trim().is_empty() {
        return Err("email inbox check failed: IMAP host is required".to_string());
    }
    if settings.imap_port == 0 {
        return Err("email inbox check failed: IMAP port is required".to_string());
    }
    if settings.username.trim().is_empty() {
        return Err("email inbox check failed: IMAP username is required".to_string());
    }
    if settings.mailbox_folder.trim().is_empty() {
        return Err("email inbox check failed: mailbox folder is required".to_string());
    }
    if secret.trim().is_empty() {
        return Err("email inbox check failed: IMAP secret is required".to_string());
    }
    Ok(())
}

fn send_provider_client_identity<T: std::io::Read + std::io::Write>(
    session: &mut imap::Session<T>,
    settings: &EmailInboundSettings,
) -> Result<(), String> {
    if !requires_imap_client_identity(&settings.provider) {
        return Ok(());
    }
    let command = format!(
        "ID (\"name\" \"Moss\" \"version\" \"{}\" \"vendor\" \"Moss\")",
        env!("CARGO_PKG_VERSION")
    );
    session
        .run_command_and_check_ok(command)
        .map_err(|error| format!("email inbox check failed: IMAP client identity failed: {error}"))
}

fn requires_imap_client_identity(provider: &EmailSenderProvider) -> bool {
    matches!(
        provider,
        EmailSenderProvider::Mail126 | EmailSenderProvider::Mail163
    )
}

fn connection_mode_for_security(security: &EmailInboundSecurity) -> ConnectionMode {
    match security {
        EmailInboundSecurity::SslTls => ConnectionMode::Tls,
        EmailInboundSecurity::StartTls => ConnectionMode::StartTls,
        EmailInboundSecurity::None => ConnectionMode::Plaintext,
    }
}

fn format_uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_raw_email_message(
    uid: Option<u32>,
    raw_body: &[u8],
    received_at: Option<String>,
) -> Result<EmailInboundMessage, String> {
    let parsed = mailparse::parse_mail(raw_body)
        .map_err(|error| format!("email inbox check failed: failed to parse email: {error}"))?;
    let headers = parsed
        .headers
        .iter()
        .map(|header| (header.get_key(), header.get_value()))
        .collect::<BTreeMap<_, _>>();
    let text_body = extract_plain_text_body(&parsed)
        .or_else(|| parsed.get_body().ok())
        .unwrap_or_default();
    let message_id = parsed.headers.get_first_value("Message-ID");
    let references = parsed
        .headers
        .get_first_value("References")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(EmailInboundMessage {
        uid: uid.map(|value| value.to_string()),
        message_id,
        from: parsed.headers.get_first_value("From").unwrap_or_default(),
        subject: parsed
            .headers
            .get_first_value("Subject")
            .unwrap_or_default(),
        text_body,
        in_reply_to: parsed.headers.get_first_value("In-Reply-To"),
        references,
        headers,
        auto_submitted: parsed.headers.get_first_value("Auto-Submitted"),
        received_at: received_at.or_else(|| parsed.headers.get_first_value("Date")),
    })
}

fn extract_plain_text_body(parsed: &ParsedMail<'_>) -> Option<String> {
    if parsed.ctype.mimetype.eq_ignore_ascii_case("text/plain") {
        return parsed.get_body().ok();
    }
    parsed.subparts.iter().find_map(extract_plain_text_body)
}

fn read_ledger(path: &Path) -> Result<EmailSessionLedger, String> {
    read_json_file::<EmailSessionLedger>(path).map(|value| value.unwrap_or_default())
}

fn write_ledger(path: &Path, ledger: &EmailSessionLedger) -> Result<(), String> {
    write_json_file(path, ledger)
}

pub(crate) fn get_listener_status(
    settings: &EmailInboundSettings,
    ledger_path: &Path,
) -> Result<EmailInboundListenerStatus, String> {
    let ledger = read_ledger(ledger_path)?;
    Ok(listener_status(settings, &ledger))
}

pub(crate) fn list_mail_sessions(
    settings: &EmailInboundSettings,
    ledger_path: &Path,
) -> Result<EmailMailSessionList, String> {
    let ledger = read_ledger(ledger_path)?;
    Ok(project_mail_sessions(settings, &ledger))
}

pub(crate) fn prepare_outgoing_continuation(
    ledger_path: &Path,
    registration: OutgoingContinuationRegistration,
    base_text_body: &str,
) -> Result<PreparedOutgoingContinuation, String> {
    let mut ledger = read_ledger(ledger_path)?;
    let now = now_rfc3339();
    let session_enabled = ledger
        .sessions
        .iter()
        .find(|session| session.session_id == registration.session_id)
        .map(|session| session.mail_driven_enabled && session.state != MailSessionState::Closed)
        .unwrap_or(false);
    let actionable = registration.actionable || session_enabled;
    let subject_tag = subject_tag_for_session(&registration.session_id);

    if actionable {
        for existing in ledger.outgoing.iter_mut().filter(|record| {
            record.session_id == registration.session_id
                && record.status == OutgoingMailStatus::Actionable
        }) {
            existing.status = OutgoingMailStatus::Superseded;
        }
    }

    let id = format!("out_{}", uuid::Uuid::new_v4());
    let message_id = if actionable {
        Some(format!("<moss-{}@moss.local>", uuid::Uuid::new_v4()))
    } else {
        None
    };
    let expires_at = if actionable {
        Some((Utc::now() + Duration::hours(registration.action_window_hours)).to_rfc3339())
    } else {
        None
    };
    let reply_token = if actionable {
        Some(format!("rp_{}", uuid::Uuid::new_v4()))
    } else {
        None
    };
    let reply_token_hash = reply_token.as_deref().map(hash_string);
    let signature = match (&reply_token, &expires_at) {
        (Some(token), Some(expires)) => Some(sign_context(
            token,
            &registration.session_id,
            &registration.workspace_id,
            &registration.thread_id,
            &registration.turn_id,
            token,
            expires,
        )),
        _ => None,
    };

    let status = if actionable {
        OutgoingMailStatus::Actionable
    } else {
        OutgoingMailStatus::NonActionable
    };
    let record = OutgoingMailRecord {
        id,
        message_id: message_id.clone(),
        session_id: registration.session_id.clone(),
        workspace_id: registration.workspace_id.clone(),
        workspace_name: registration.workspace_name.clone(),
        thread_id: registration.thread_id.clone(),
        thread_name: registration.thread_name.clone(),
        turn_id: registration.turn_id.clone(),
        reply_token_hash,
        expires_at: expires_at.clone(),
        status,
        sent_at: now.clone(),
        subject_tag: Some(subject_tag.clone()),
        subject: registration.subject.clone(),
        summary: registration.summary.clone(),
        next_recommendations: registration.next_recommendations.clone(),
        signature: signature.clone(),
    };

    upsert_session_control(&mut ledger, &registration.session_id, actionable, &now);
    ledger.outgoing.push(record.clone());
    write_ledger(ledger_path, &ledger)?;

    let subject = if actionable {
        format!("{subject_tag} {}", registration.subject.trim())
    } else {
        registration.subject.trim().to_string()
    };
    let text_body = if let (Some(token), Some(expires), Some(sig)) = (
        reply_token.as_deref(),
        expires_at.as_deref(),
        signature.as_deref(),
    ) {
        format!(
            "{}\n\n-- Moss Session: {} · 请勿删除此行 --\n{}\nsession: {}\nworkspace: {}\nthread: {}\nturn: {}\nreply: {}\nexpires: {}\nsig: {}\n{}",
            base_text_body.trim(),
            registration.session_id,
            MOSS_CONTEXT_START,
            registration.session_id,
            registration.workspace_id,
            registration.thread_id,
            registration.turn_id,
            token,
            expires,
            sig,
            MOSS_CONTEXT_END,
        )
    } else {
        base_text_body.trim().to_string()
    };
    let headers = if actionable {
        vec![
            ("X-Moss-Session-Id".to_string(), registration.session_id),
            ("X-Moss-Workspace-Id".to_string(), registration.workspace_id),
            ("X-Moss-Thread-Id".to_string(), registration.thread_id),
            ("X-Moss-Turn-Id".to_string(), registration.turn_id),
            (
                "X-Moss-Reply-Token".to_string(),
                reply_token.unwrap_or_default(),
            ),
            (
                "X-Moss-Expires-At".to_string(),
                expires_at.unwrap_or_default(),
            ),
            (
                "X-Moss-Signature".to_string(),
                signature.unwrap_or_default(),
            ),
        ]
    } else {
        Vec::new()
    };

    Ok(PreparedOutgoingContinuation {
        record,
        subject,
        text_body,
        message_id,
        headers,
    })
}

pub(crate) fn check_mailbox_with_reader(
    settings: &EmailInboundSettings,
    ledger_path: &Path,
    reader: &impl MailboxReader,
) -> Result<CheckEmailInboxResult, String> {
    let mut ledger = read_ledger(ledger_path)?;
    let checked_at = now_rfc3339();
    let messages = reader.read_since(&ledger.cursor.last_seen_uid)?;
    let mut accepted_count = 0;
    let mut queued_count = 0;
    let mut needs_confirmation_count = 0;
    let mut rejected_count = 0;
    let mut ignored_count = 0;
    let mut duplicate_count = 0;
    let mut latest_seen_uid = ledger.cursor.last_seen_uid.clone();

    for message in &messages {
        let outcome = process_inbound_message(settings, &mut ledger, message, &checked_at);
        match outcome.as_str() {
            "accepted" => accepted_count += 1,
            "queued" => queued_count += 1,
            "needs_confirmation" => needs_confirmation_count += 1,
            "rejected" => rejected_count += 1,
            "duplicate" => duplicate_count += 1,
            _ => ignored_count += 1,
        }
        latest_seen_uid = advance_mailbox_cursor_uid(latest_seen_uid, message.uid.as_deref());
    }

    ledger.cursor.last_seen_uid = latest_seen_uid;
    ledger.cursor.last_checked_at = Some(checked_at.clone());
    write_ledger(ledger_path, &ledger)?;

    Ok(CheckEmailInboxResult {
        checked_at,
        read_only: reader.is_read_only(),
        scanned_count: messages.len(),
        accepted_count,
        queued_count,
        needs_confirmation_count,
        rejected_count,
        ignored_count,
        duplicate_count,
    })
}

pub(crate) fn mutate_mail_session(
    ledger_path: &Path,
    request: MutateMailSessionRequest,
) -> Result<EmailMailSessionList, String> {
    let mut ledger = read_ledger(ledger_path)?;
    let now = now_rfc3339();
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return Err("email mail session mutation failed: missing session id".to_string());
    }
    match request.action.as_str() {
        "enable" => upsert_session_control(&mut ledger, session_id, true, &now),
        "pause" => set_session_state(&mut ledger, session_id, MailSessionState::Paused, &now),
        "resume" => set_session_state(&mut ledger, session_id, MailSessionState::Enabled, &now),
        "close" => {
            set_session_state(&mut ledger, session_id, MailSessionState::Closed, &now);
            for outgoing in ledger.outgoing.iter_mut().filter(|record| {
                record.session_id == session_id && record.status == OutgoingMailStatus::Actionable
            }) {
                outgoing.status = OutgoingMailStatus::Closed;
            }
        }
        "confirm" => {
            let command_id = request.command_id.as_deref().ok_or_else(|| {
                "email command confirmation failed: missing command id".to_string()
            })?;
            if let Some(command) = ledger.commands.iter_mut().find(|command| {
                command.id == command_id
                    && command.status == InboundCommandStatus::NeedsConfirmation
            }) {
                command.status = InboundCommandStatus::Queued;
            }
        }
        "ignore" => {
            let command_id = request
                .command_id
                .as_deref()
                .ok_or_else(|| "email command ignore failed: missing command id".to_string())?;
            if let Some(command) = ledger
                .commands
                .iter_mut()
                .find(|command| command.id == command_id)
            {
                command.status = InboundCommandStatus::Ignored;
            }
        }
        "cleanup" => {
            ledger.commands.retain(|command| {
                !matches!(
                    command.status,
                    InboundCommandStatus::Done
                        | InboundCommandStatus::Duplicate
                        | InboundCommandStatus::Expired
                        | InboundCommandStatus::Ignored
                )
            });
        }
        "delete_mail_records" => {
            ledger
                .outgoing
                .retain(|record| record.session_id != session_id);
            ledger
                .commands
                .retain(|command| command.session_id != session_id);
        }
        _ => {
            return Err(format!(
                "unsupported mail session action: {}",
                request.action
            ))
        }
    }
    write_ledger(ledger_path, &ledger)?;
    Ok(project_mail_sessions(
        &EmailInboundSettings::default(),
        &ledger,
    ))
}

pub(crate) fn claim_next_command(ledger_path: &Path) -> Result<ClaimMailCommandResult, String> {
    let mut ledger = read_ledger(ledger_path)?;
    let command = ledger
        .commands
        .iter_mut()
        .find(|command| command.status == InboundCommandStatus::Queued)
        .map(|command| {
            command.status = InboundCommandStatus::Running;
            command.clone()
        });
    write_ledger(ledger_path, &ledger)?;
    Ok(ClaimMailCommandResult { command })
}

pub(crate) fn complete_command(
    ledger_path: &Path,
    request: CompleteMailCommandRequest,
) -> Result<EmailMailSessionList, String> {
    let mut ledger = read_ledger(ledger_path)?;
    if let Some(command) = ledger
        .commands
        .iter_mut()
        .find(|command| command.id == request.command_id)
    {
        command.status = request.status;
        command.reject_reason = request.reject_reason;
    }
    write_ledger(ledger_path, &ledger)?;
    Ok(project_mail_sessions(
        &EmailInboundSettings::default(),
        &ledger,
    ))
}

pub(crate) fn parse_reply_command(reply_text: &str) -> ParsedReplyCommand {
    let user_text = strip_quoted_reply_text(reply_text);
    let effective_lines = user_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if effective_lines.is_empty() {
        return ParsedReplyCommand::NeedsConfirmation("missing_action".to_string());
    }

    if let Some(action) = parse_natural_reply_action(&effective_lines) {
        return ParsedReplyCommand::Command {
            action,
            detail: None,
        };
    }

    let action_lines = effective_lines
        .iter()
        .filter(|line| line.to_ascii_uppercase().starts_with("ACTION:"))
        .copied()
        .collect::<Vec<_>>();
    if action_lines.is_empty() {
        let detail = extract_detail(&user_text).unwrap_or_else(|| user_text.clone());
        let detail = detail.trim().to_string();
        if detail.is_empty() {
            return ParsedReplyCommand::NeedsConfirmation("missing_action".to_string());
        }
        if is_high_risk_or_out_of_scope(&detail) {
            return ParsedReplyCommand::NeedsConfirmation(
                "high_risk_or_out_of_scope_detail".to_string(),
            );
        }
        return ParsedReplyCommand::Command {
            action: EmailMailCommandAction::Change,
            detail: Some(detail),
        };
    }
    if action_lines.len() != 1 {
        return ParsedReplyCommand::NeedsConfirmation("multiple_actions".to_string());
    }

    let action = match action_lines[0]
        .split_once(':')
        .map(|(_, value)| value.trim().to_ascii_uppercase())
        .as_deref()
    {
        Some("NEXT") => EmailMailCommandAction::Next,
        Some("CHANGE") => EmailMailCommandAction::Change,
        Some("PAUSE") => EmailMailCommandAction::Pause,
        Some("STOP") => EmailMailCommandAction::Stop,
        Some("STATUS") => EmailMailCommandAction::Status,
        _ => return ParsedReplyCommand::NeedsConfirmation("invalid_action".to_string()),
    };

    let detail = extract_detail(&user_text);
    if action == EmailMailCommandAction::Change {
        let Some(detail) = detail.filter(|value| !value.trim().is_empty()) else {
            return ParsedReplyCommand::NeedsConfirmation("missing_change_detail".to_string());
        };
        if is_high_risk_or_out_of_scope(&detail) {
            return ParsedReplyCommand::NeedsConfirmation(
                "high_risk_or_out_of_scope_detail".to_string(),
            );
        }
        return ParsedReplyCommand::Command {
            action,
            detail: Some(detail),
        };
    }

    ParsedReplyCommand::Command {
        action,
        detail: None,
    }
}

fn parse_natural_reply_action(lines: &[&str]) -> Option<EmailMailCommandAction> {
    if lines.len() != 1 {
        return None;
    }
    let normalized = normalize_reply_action_text(lines[0]);
    match normalized.as_str() {
        "@moss continue" | "continue" | "next" | "go" | "继续" | "继续执行" | "下一步"
        | "执行下一步" | "按建议继续" => Some(EmailMailCommandAction::Next),
        "@moss stop" | "stop" | "停止" | "关闭" | "结束" => {
            Some(EmailMailCommandAction::Stop)
        }
        "pause" | "暂停" | "先暂停" => Some(EmailMailCommandAction::Pause),
        "status" | "状态" | "进度" | "查看状态" => Some(EmailMailCommandAction::Status),
        _ => None,
    }
}

fn normalize_reply_action_text(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '.' | '。' | ',' | '，' | '!' | '！' | '?' | '？' | ':' | '：'
            )
        })
        .replace(':', " ")
        .replace('：', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub(crate) fn strip_quoted_reply_text(text: &str) -> String {
    let above_delimiter = text.split(EMAIL_REPLY_DELIMITER).next().unwrap_or(text);
    let mut kept = Vec::new();
    for raw_line in above_delimiter.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.starts_with('>') {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("on ") && lower.ends_with(" wrote:") {
            break;
        }
        if is_quoted_reply_header(trimmed) {
            break;
        }
        if lower.contains("forwarded message") {
            break;
        }
        if trimmed == "--" || trimmed == "-- " {
            break;
        }
        kept.push(line);
    }
    kept.join("\n").trim().to_string()
}

fn is_quoted_reply_header(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("from:")
        || lower.starts_with("sent:")
        || lower.starts_with("to:")
        || lower.starts_with("subject:")
        || line.starts_with("发件人:")
        || line.starts_with("发件人：")
        || line.starts_with("时间:")
        || line.starts_with("时间：")
        || line.starts_with("主题:")
        || line.starts_with("主题：")
        || line.starts_with("收件人:")
        || line.starts_with("收件人：")
}

fn process_inbound_message(
    settings: &EmailInboundSettings,
    ledger: &mut EmailSessionLedger,
    message: &EmailInboundMessage,
    now: &str,
) -> String {
    if is_auto_reply_or_bounce(message) {
        return "ignored".to_string();
    }

    let parsed_context = parse_moss_context(message);
    let candidate_index = find_candidate_outgoing(ledger, message, &parsed_context);
    let is_moss_like = candidate_index.is_some() || parsed_context.session_id.is_some();
    if !is_moss_like {
        return "ignored".to_string();
    }

    if !is_allowed_sender(settings, &message.from) {
        push_rejected_command(
            ledger,
            message,
            candidate_index,
            now,
            "sender_not_allowed",
            InboundCommandStatus::Rejected,
        );
        return "rejected".to_string();
    }

    let Some(outgoing_index) = candidate_index else {
        push_rejected_command(
            ledger,
            message,
            None,
            now,
            "missing_outgoing_context",
            InboundCommandStatus::NeedsConfirmation,
        );
        return "needs_confirmation".to_string();
    };

    let outgoing = ledger.outgoing[outgoing_index].clone();
    if outgoing.status != OutgoingMailStatus::Actionable {
        let status = if outgoing.status == OutgoingMailStatus::Superseded {
            "stale_reply_token"
        } else {
            "mail_session_not_actionable"
        };
        push_rejected_command(
            ledger,
            message,
            Some(outgoing_index),
            now,
            status,
            InboundCommandStatus::Rejected,
        );
        return "rejected".to_string();
    }
    let reply_token = parsed_context
        .reply_token
        .clone()
        .or_else(|| header_value(message, "X-Moss-Reply-Token"));
    let reply_token_hash = match reply_token.as_deref() {
        Some(token) => {
            let token_hash = hash_string(token);
            if outgoing.reply_token_hash.as_deref() != Some(token_hash.as_str()) {
                push_rejected_command(
                    ledger,
                    message,
                    Some(outgoing_index),
                    now,
                    "reply_token_mismatch",
                    InboundCommandStatus::Rejected,
                );
                return "rejected".to_string();
            }
            token_hash
        }
        None => outgoing
            .reply_token_hash
            .clone()
            .unwrap_or_else(|| hash_string(&outgoing.id)),
    };
    if is_expired(outgoing.expires_at.as_deref(), now) {
        ledger.outgoing[outgoing_index].status = OutgoingMailStatus::Expired;
        push_rejected_command(
            ledger,
            message,
            Some(outgoing_index),
            now,
            "expired_reply_token",
            InboundCommandStatus::Expired,
        );
        return "rejected".to_string();
    }
    if let Some(reply_token) = reply_token.as_deref() {
        let expected_signature = sign_context(
            reply_token,
            &outgoing.session_id,
            &outgoing.workspace_id,
            &outgoing.thread_id,
            &outgoing.turn_id,
            reply_token,
            outgoing.expires_at.as_deref().unwrap_or_default(),
        );
        let actual_signature = parsed_context
            .signature
            .clone()
            .or_else(|| header_value(message, "X-Moss-Signature"));
        if actual_signature.as_deref() != Some(expected_signature.as_str()) {
            push_rejected_command(
                ledger,
                message,
                Some(outgoing_index),
                now,
                "signature_mismatch",
                InboundCommandStatus::Rejected,
            );
            return "rejected".to_string();
        }
    }

    let user_reply = strip_quoted_reply_text(&message.text_body);
    let body_hash = hash_string(&user_reply);
    let mail_message_id = normalized_mail_message_id(message);
    if ledger.commands.iter().any(|command| {
        command.mail_message_id == mail_message_id
            && command.reply_token_hash == reply_token_hash
            && command.body_hash == body_hash
    }) {
        push_duplicate_command(
            ledger,
            &outgoing,
            message,
            &mail_message_id,
            &reply_token_hash,
            &body_hash,
            now,
        );
        return "duplicate".to_string();
    }

    match parse_reply_command(&user_reply) {
        ParsedReplyCommand::NeedsConfirmation(reason) => {
            let command = build_command(
                &outgoing,
                message,
                &mail_message_id,
                &reply_token_hash,
                &body_hash,
                EmailMailCommandAction::Change,
                Some(sanitize_detail(&user_reply)),
                InboundCommandStatus::NeedsConfirmation,
                Some(reason),
                now,
            );
            ledger.commands.push(command);
            "needs_confirmation".to_string()
        }
        ParsedReplyCommand::Command { action, detail } => {
            let mut status = InboundCommandStatus::Queued;
            let mut effective_detail = detail;
            match action {
                EmailMailCommandAction::Next => {
                    effective_detail = outgoing.next_recommendations.first().cloned();
                    if effective_detail.as_deref().unwrap_or("").trim().is_empty() {
                        status = InboundCommandStatus::NeedsConfirmation;
                    }
                }
                EmailMailCommandAction::Pause => {
                    set_session_state(ledger, &outgoing.session_id, MailSessionState::Paused, now);
                    status = InboundCommandStatus::Done;
                }
                EmailMailCommandAction::Stop => {
                    set_session_state(ledger, &outgoing.session_id, MailSessionState::Closed, now);
                    ledger.outgoing[outgoing_index].status = OutgoingMailStatus::Closed;
                    status = InboundCommandStatus::Done;
                }
                EmailMailCommandAction::Status => {
                    status = InboundCommandStatus::Done;
                }
                EmailMailCommandAction::Change => {}
            }
            let outcome = if status == InboundCommandStatus::Queued {
                "queued"
            } else if status == InboundCommandStatus::NeedsConfirmation {
                "needs_confirmation"
            } else {
                "accepted"
            };
            let command = build_command(
                &outgoing,
                message,
                &mail_message_id,
                &reply_token_hash,
                &body_hash,
                action,
                effective_detail,
                status,
                None,
                now,
            );
            ledger.commands.push(command);
            outcome.to_string()
        }
    }
}

fn build_command(
    outgoing: &OutgoingMailRecord,
    message: &EmailInboundMessage,
    mail_message_id: &str,
    reply_token_hash: &str,
    body_hash: &str,
    action: EmailMailCommandAction,
    detail: Option<String>,
    status: InboundCommandStatus,
    reject_reason: Option<String>,
    now: &str,
) -> InboundMailCommand {
    InboundMailCommand {
        id: format!("cmd_{}", uuid::Uuid::new_v4()),
        mail_message_id: mail_message_id.to_string(),
        in_reply_to: message.in_reply_to.clone(),
        linked_outgoing_mail_id: outgoing.id.clone(),
        session_id: outgoing.session_id.clone(),
        workspace_id: outgoing.workspace_id.clone(),
        thread_id: outgoing.thread_id.clone(),
        turn_id: outgoing.turn_id.clone(),
        reply_token_hash: reply_token_hash.to_string(),
        from_hash: hash_string(&message.from.to_ascii_lowercase()),
        from_display: Some(redact_email(&message.from)),
        received_at: message
            .received_at
            .clone()
            .unwrap_or_else(|| now.to_string()),
        action,
        detail: detail.map(|value| sanitize_detail(&value)),
        body_hash: body_hash.to_string(),
        status,
        reject_reason,
        subject_tag: outgoing.subject_tag.clone(),
    }
}

fn push_duplicate_command(
    ledger: &mut EmailSessionLedger,
    outgoing: &OutgoingMailRecord,
    message: &EmailInboundMessage,
    mail_message_id: &str,
    reply_token_hash: &str,
    body_hash: &str,
    now: &str,
) {
    ledger.commands.push(build_command(
        outgoing,
        message,
        mail_message_id,
        reply_token_hash,
        body_hash,
        EmailMailCommandAction::Status,
        None,
        InboundCommandStatus::Duplicate,
        Some("duplicate_reply".to_string()),
        now,
    ));
}

fn push_rejected_command(
    ledger: &mut EmailSessionLedger,
    message: &EmailInboundMessage,
    outgoing_index: Option<usize>,
    now: &str,
    reason: &str,
    status: InboundCommandStatus,
) {
    let fallback = OutgoingMailRecord {
        id: "unknown".to_string(),
        message_id: None,
        session_id: "unknown".to_string(),
        workspace_id: "unknown".to_string(),
        workspace_name: None,
        thread_id: "unknown".to_string(),
        thread_name: None,
        turn_id: "unknown".to_string(),
        reply_token_hash: Some(hash_string("unknown")),
        expires_at: None,
        status: OutgoingMailStatus::Closed,
        sent_at: now.to_string(),
        subject_tag: None,
        subject: sanitize_detail(&message.subject),
        summary: String::new(),
        next_recommendations: Vec::new(),
        signature: None,
    };
    let outgoing = outgoing_index
        .and_then(|index| ledger.outgoing.get(index))
        .unwrap_or(&fallback);
    let command = build_command(
        outgoing,
        message,
        &normalized_mail_message_id(message),
        outgoing.reply_token_hash.as_deref().unwrap_or("unknown"),
        &hash_string(&strip_quoted_reply_text(&message.text_body)),
        EmailMailCommandAction::Status,
        None,
        status,
        Some(reason.to_string()),
        now,
    );
    ledger.commands.push(command);
}

fn find_candidate_outgoing(
    ledger: &EmailSessionLedger,
    message: &EmailInboundMessage,
    context: &ParsedMossContext,
) -> Option<usize> {
    if let Some(index) = message
        .in_reply_to
        .as_deref()
        .and_then(|value| find_outgoing_by_message_id(ledger, value))
    {
        return Some(index);
    }
    for reference in &message.references {
        if let Some(index) = find_outgoing_by_message_id(ledger, reference) {
            return Some(index);
        }
    }
    if let Some(token) = context
        .reply_token
        .clone()
        .or_else(|| header_value(message, "X-Moss-Reply-Token"))
    {
        let token_hash = hash_string(&token);
        if let Some((index, _)) = ledger
            .outgoing
            .iter()
            .enumerate()
            .find(|(_, record)| record.reply_token_hash.as_deref() == Some(token_hash.as_str()))
        {
            return Some(index);
        }
    }
    if let Some(session_id) = context
        .session_id
        .clone()
        .or_else(|| header_value(message, "X-Moss-Session-Id"))
    {
        return ledger
            .outgoing
            .iter()
            .enumerate()
            .rev()
            .find(|(_, record)| record.session_id == session_id)
            .map(|(index, _)| index);
    }
    if let Some(tag) = extract_subject_tag(&message.subject) {
        return ledger
            .outgoing
            .iter()
            .enumerate()
            .rev()
            .find(|(_, record)| record.subject_tag.as_deref() == Some(tag.as_str()))
            .map(|(index, _)| index);
    }
    None
}

fn find_outgoing_by_message_id(ledger: &EmailSessionLedger, message_id: &str) -> Option<usize> {
    let normalized = message_id.trim();
    ledger
        .outgoing
        .iter()
        .enumerate()
        .find(|(_, record)| record.message_id.as_deref() == Some(normalized))
        .map(|(index, _)| index)
}

fn parse_moss_context(message: &EmailInboundMessage) -> ParsedMossContext {
    let mut context = ParsedMossContext {
        session_id: header_value(message, "X-Moss-Session-Id"),
        workspace_id: header_value(message, "X-Moss-Workspace-Id"),
        thread_id: header_value(message, "X-Moss-Thread-Id"),
        turn_id: header_value(message, "X-Moss-Turn-Id"),
        reply_token: header_value(message, "X-Moss-Reply-Token"),
        expires_at: header_value(message, "X-Moss-Expires-At"),
        signature: header_value(message, "X-Moss-Signature"),
    };

    for line in context_block(&message.text_body).lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "session" => context.session_id.get_or_insert(value),
            "workspace" => context.workspace_id.get_or_insert(value),
            "thread" => context.thread_id.get_or_insert(value),
            "turn" => context.turn_id.get_or_insert(value),
            "reply" => context.reply_token.get_or_insert(value),
            "expires" => context.expires_at.get_or_insert(value),
            "sig" => context.signature.get_or_insert(value),
            _ => continue,
        };
    }
    if context.session_id.is_none() {
        context.session_id = extract_body_anchor(&message.text_body);
    }
    context
}

fn context_block(text: &str) -> String {
    let Some((_, after_start)) = text.split_once(MOSS_CONTEXT_START) else {
        return String::new();
    };
    after_start
        .split_once(MOSS_CONTEXT_END)
        .map(|(block, _)| block)
        .unwrap_or(after_start)
        .trim()
        .to_string()
}

fn extract_body_anchor(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("-- Moss Session:")
            .and_then(|rest| rest.split(['·', '-']).next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn header_value(message: &EmailInboundMessage, header_name: &str) -> Option<String> {
    message.headers.iter().find_map(|(key, value)| {
        if key.eq_ignore_ascii_case(header_name) {
            Some(value.trim().to_string()).filter(|value| !value.is_empty())
        } else {
            None
        }
    })
}

fn extract_subject_tag(subject: &str) -> Option<String> {
    let start = subject.find("[Moss #")?;
    let tail = &subject[start..];
    let end = tail.find(']')?;
    Some(tail[..=end].to_string())
}

fn is_allowed_sender(settings: &EmailInboundSettings, from: &str) -> bool {
    let sender = extract_email_address(from).to_ascii_lowercase();
    settings
        .allowed_senders
        .iter()
        .any(|allowed| extract_email_address(allowed).eq_ignore_ascii_case(&sender))
}

fn normalize_allowed_senders(configured: &[String], fallback_recipient: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let fallback = fallback_recipient.to_string();
    let mut allowed = configured
        .iter()
        .chain(std::iter::once(&fallback))
        .filter_map(|value| {
            let normalized = extract_email_address(value);
            if normalized.is_empty() || !seen.insert(normalized.to_ascii_lowercase()) {
                None
            } else {
                Some(normalized)
            }
        })
        .collect::<Vec<_>>();
    allowed.sort();
    allowed
}

fn extract_email_address(value: &str) -> String {
    let trimmed = value.trim();
    if let (Some(start), Some(end)) = (trimmed.find('<'), trimmed.find('>')) {
        if start < end {
            return trimmed[start + 1..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

fn advance_mailbox_cursor_uid(current: Option<String>, next: Option<&str>) -> Option<String> {
    let next = next.map(str::trim).filter(|value| !value.is_empty())?;
    let Some(current_value) = current
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Some(next.to_string());
    };
    match (current_value.parse::<u32>(), next.parse::<u32>()) {
        (Ok(current_number), Ok(next_number)) => {
            if next_number > current_number {
                Some(next.to_string())
            } else {
                Some(current_value.to_string())
            }
        }
        _ => Some(next.to_string()),
    }
}

fn is_auto_reply_or_bounce(message: &EmailInboundMessage) -> bool {
    if message
        .auto_submitted
        .as_deref()
        .map(|value| !value.eq_ignore_ascii_case("no"))
        .unwrap_or(false)
    {
        return true;
    }
    let subject = message.subject.to_ascii_lowercase();
    if subject.contains("delivery status notification")
        || subject.contains("undeliverable")
        || subject.contains("out of office")
        || subject.contains("automatic reply")
    {
        return true;
    }
    message.headers.iter().any(|(key, value)| {
        let key = key.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();
        (key == "auto-submitted" && value != "no")
            || (key == "precedence" && (value == "bulk" || value == "junk" || value == "list"))
            || key == "x-autoreply"
            || key == "x-auto-response-suppress"
    })
}

fn extract_detail(text: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut in_detail = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.to_ascii_uppercase().starts_with("DETAIL:") {
            in_detail = true;
            let value = trimmed
                .split_once(':')
                .map(|(_, value)| value.trim())
                .unwrap_or("");
            if !value.is_empty() {
                lines.push(value.to_string());
            }
            continue;
        }
        if in_detail {
            if trimmed.to_ascii_uppercase().starts_with("ACTION:") {
                break;
            }
            lines.push(line.trim_end().to_string());
        }
    }
    let detail = lines.join("\n").trim().to_string();
    if detail.is_empty() {
        None
    } else {
        Some(detail)
    }
}

fn is_high_risk_or_out_of_scope(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    [
        "rm -rf",
        "delete all",
        "format disk",
        "reset --hard",
        "drop database",
        "steal",
        "password",
        "secret",
        "private key",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_expired(expires_at: Option<&str>, now: &str) -> bool {
    let Some(expires_at) = expires_at else {
        return true;
    };
    let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) else {
        return true;
    };
    let Ok(now) = chrono::DateTime::parse_from_rfc3339(now) else {
        return true;
    };
    now > expires
}

fn upsert_session_control(
    ledger: &mut EmailSessionLedger,
    session_id: &str,
    mail_driven_enabled: bool,
    now: &str,
) {
    if let Some(session) = ledger
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    {
        if mail_driven_enabled {
            session.mail_driven_enabled = true;
            if session.state == MailSessionState::Closed {
                session.state = MailSessionState::Enabled;
            }
        }
        session.updated_at = now.to_string();
        return;
    }
    ledger.sessions.push(MailSessionControl {
        session_id: session_id.to_string(),
        state: MailSessionState::Enabled,
        mail_driven_enabled,
        updated_at: now.to_string(),
    });
}

fn set_session_state(
    ledger: &mut EmailSessionLedger,
    session_id: &str,
    state: MailSessionState,
    now: &str,
) {
    upsert_session_control(ledger, session_id, true, now);
    if let Some(session) = ledger
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    {
        session.state = state;
        session.updated_at = now.to_string();
    }
}

fn project_mail_sessions(
    settings: &EmailInboundSettings,
    ledger: &EmailSessionLedger,
) -> EmailMailSessionList {
    let listener = listener_status(settings, ledger);
    let mut session_ids = ledger
        .outgoing
        .iter()
        .map(|record| record.session_id.clone())
        .chain(
            ledger
                .commands
                .iter()
                .map(|command| command.session_id.clone()),
        )
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    session_ids.sort();
    let mut sessions = Vec::new();
    for session_id in session_ids {
        let latest_outgoing = ledger
            .outgoing
            .iter()
            .filter(|record| record.session_id == session_id)
            .max_by_key(|record| record.sent_at.clone());
        let latest_command = ledger
            .commands
            .iter()
            .filter(|command| command.session_id == session_id)
            .max_by_key(|command| command.received_at.clone());
        let state = ledger
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .map(|session| session.state.clone())
            .unwrap_or(MailSessionState::Enabled);
        let outbound_count = ledger
            .outgoing
            .iter()
            .filter(|record| record.session_id == session_id)
            .count();
        let inbound_count = ledger
            .commands
            .iter()
            .filter(|command| command.session_id == session_id)
            .count();
        let queued_count = ledger
            .commands
            .iter()
            .filter(|command| {
                command.session_id == session_id && command.status == InboundCommandStatus::Queued
            })
            .count();
        let needs_confirmation_count = ledger
            .commands
            .iter()
            .filter(|command| {
                command.session_id == session_id
                    && command.status == InboundCommandStatus::NeedsConfirmation
            })
            .count();
        sessions.push(EmailMailSessionRow {
            session_id: session_id.clone(),
            workspace_id: latest_outgoing
                .map(|record| record.workspace_id.clone())
                .or_else(|| latest_command.map(|command| command.workspace_id.clone()))
                .unwrap_or_default(),
            thread_id: latest_outgoing
                .map(|record| record.thread_id.clone())
                .or_else(|| latest_command.map(|command| command.thread_id.clone()))
                .unwrap_or_default(),
            turn_id: latest_outgoing
                .map(|record| record.turn_id.clone())
                .or_else(|| latest_command.map(|command| command.turn_id.clone()))
                .unwrap_or_default(),
            workspace_name: latest_outgoing.and_then(|record| record.workspace_name.clone()),
            thread_name: latest_outgoing.and_then(|record| record.thread_name.clone()),
            state,
            last_event_at: latest_command
                .map(|command| command.received_at.clone())
                .or_else(|| latest_outgoing.map(|record| record.sent_at.clone())),
            latest_action: latest_command.map(|command| command.action.clone()),
            latest_status: latest_command.map(|command| command.status.clone()),
            latest_reject_reason: latest_command.and_then(|command| command.reject_reason.clone()),
            outbound_count,
            inbound_count,
            queued_count,
            needs_confirmation_count,
            latest_summary: latest_outgoing.map(|record| record.summary.clone()),
        });
    }
    sessions.sort_by(|left, right| right.last_event_at.cmp(&left.last_event_at));
    EmailMailSessionList {
        listener,
        sessions,
        timeline: project_timeline(ledger),
    }
}

fn project_timeline(ledger: &EmailSessionLedger) -> Vec<EmailMailTimelineEvent> {
    let mut timeline = Vec::new();
    for outgoing in &ledger.outgoing {
        timeline.push(EmailMailTimelineEvent {
            id: outgoing.id.clone(),
            session_id: outgoing.session_id.clone(),
            direction: "outbound".to_string(),
            action: None,
            status: format!("{:?}", outgoing.status).to_ascii_lowercase(),
            subject: Some(outgoing.subject.clone()),
            detail: Some(sanitize_detail(&outgoing.summary)),
            reject_reason: None,
            occurred_at: outgoing.sent_at.clone(),
        });
    }
    for command in &ledger.commands {
        timeline.push(EmailMailTimelineEvent {
            id: command.id.clone(),
            session_id: command.session_id.clone(),
            direction: "inbound".to_string(),
            action: Some(command.action.clone()),
            status: format!("{:?}", command.status).to_ascii_lowercase(),
            subject: None,
            detail: command.detail.clone(),
            reject_reason: command.reject_reason.clone(),
            occurred_at: command.received_at.clone(),
        });
    }
    timeline.sort_by(|left, right| right.occurred_at.cmp(&left.occurred_at));
    timeline
}

fn listener_status(
    settings: &EmailInboundSettings,
    ledger: &EmailSessionLedger,
) -> EmailInboundListenerStatus {
    let last_checked_at = ledger.cursor.last_checked_at.clone();
    let next_check_at = last_checked_at.as_deref().and_then(|value| {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|time| {
                (time + Duration::seconds(settings.poll_interval_seconds as i64)).to_rfc3339()
            })
    });
    EmailInboundListenerStatus {
        enabled: settings.enabled,
        read_only: true,
        connection_state: if settings.enabled {
            "ready".to_string()
        } else {
            "paused".to_string()
        },
        last_checked_at,
        next_check_at,
        accepted_count: ledger
            .commands
            .iter()
            .filter(|command| command.status == InboundCommandStatus::Accepted)
            .count(),
        queued_count: ledger
            .commands
            .iter()
            .filter(|command| command.status == InboundCommandStatus::Queued)
            .count(),
        needs_confirmation_count: ledger
            .commands
            .iter()
            .filter(|command| command.status == InboundCommandStatus::NeedsConfirmation)
            .count(),
        rejected_count: ledger
            .commands
            .iter()
            .filter(|command| {
                matches!(
                    command.status,
                    InboundCommandStatus::Rejected | InboundCommandStatus::Expired
                )
            })
            .count(),
        ignored_count: ledger
            .commands
            .iter()
            .filter(|command| command.status == InboundCommandStatus::Ignored)
            .count(),
        polling_interval_seconds: settings.poll_interval_seconds,
    }
}

fn normalized_mail_message_id(message: &EmailInboundMessage) -> String {
    message
        .message_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("missing-message-id:{}", hash_string(&message.text_body)))
}

fn subject_tag_for_session(session_id: &str) -> String {
    let compact = session_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(18)
        .collect::<String>();
    format!(
        "[Moss #{}]",
        if compact.is_empty() {
            "session"
        } else {
            &compact
        }
    )
}

fn sign_context(
    key: &str,
    session_id: &str,
    workspace_id: &str,
    thread_id: &str,
    turn_id: &str,
    reply_token: &str,
    expires_at: &str,
) -> String {
    let canonical = format!(
        "session={session_id}\nworkspace={workspace_id}\nthread={thread_id}\nturn={turn_id}\nreply={reply_token}\nexpires={expires_at}"
    );
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

fn hash_string(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn sanitize_detail(value: &str) -> String {
    let compact = value
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if compact.chars().count() <= 1000 {
        compact
    } else {
        let truncated = compact.chars().take(1000).collect::<String>();
        format!("{truncated}...[truncated]")
    }
}

fn redact_email(value: &str) -> String {
    let address = extract_email_address(value);
    let Some((name, domain)) = address.split_once('@') else {
        return "unknown".to_string();
    };
    let prefix = name.chars().next().unwrap_or('*');
    format!("{prefix}***@{domain}")
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> EmailInboundSettings {
        normalize_inbound_settings(
            &EmailInboundSettings {
                enabled: true,
                allowed_senders: vec!["user@example.com".to_string()],
                ..EmailInboundSettings::default()
            },
            "",
        )
    }

    #[test]
    fn netease_imap_providers_require_client_identity() {
        assert!(requires_imap_client_identity(&EmailSenderProvider::Mail126));
        assert!(requires_imap_client_identity(&EmailSenderProvider::Mail163));
        assert!(!requires_imap_client_identity(&EmailSenderProvider::Qq));
        assert!(!requires_imap_client_identity(&EmailSenderProvider::Custom));
    }

    #[test]
    fn inbound_settings_preserve_ten_second_poll_interval() {
        let normalized = normalize_inbound_settings(
            &EmailInboundSettings {
                poll_interval_seconds: 10,
                ..EmailInboundSettings::default()
            },
            "",
        );

        assert_eq!(normalized.poll_interval_seconds, 10);
    }

    fn temp_ledger_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "moss-email-session-{name}-{}.json",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn sanitize_detail_truncates_multibyte_text_without_panicking() {
        let detail = "可以按这 6 个方向推进（按优先级）：\n策".repeat(80);
        let sanitized = sanitize_detail(&detail);

        assert!(sanitized.ends_with("...[truncated]"));
        assert!(sanitized.chars().count() <= 1014);
    }

    #[test]
    fn malformed_email_address_brackets_do_not_panic() {
        assert_eq!(
            extract_email_address("User <user@example.com>"),
            "user@example.com"
        );
        assert_eq!(extract_email_address("bad> <"), "bad> <");
        assert_eq!(extract_email_address("<missing-end"), "<missing-end");
    }

    #[test]
    fn mailbox_cursor_uses_highest_numeric_uid_from_unordered_reader() {
        let path = temp_ledger_path("cursor-highest");
        let older = EmailInboundMessage {
            uid: Some("9".to_string()),
            message_id: Some("<old@example.com>".to_string()),
            from: "other@example.com".to_string(),
            subject: "old".to_string(),
            text_body: "not moss".to_string(),
            in_reply_to: None,
            references: Vec::new(),
            headers: BTreeMap::new(),
            auto_submitted: None,
            received_at: None,
        };
        let newer = EmailInboundMessage {
            uid: Some("10".to_string()),
            message_id: Some("<new@example.com>".to_string()),
            from: "other@example.com".to_string(),
            subject: "new".to_string(),
            text_body: "not moss".to_string(),
            in_reply_to: None,
            references: Vec::new(),
            headers: BTreeMap::new(),
            auto_submitted: None,
            received_at: None,
        };

        check_mailbox_with_reader(
            &settings(),
            &path,
            &MemoryMailboxReader::new(vec![newer, older]),
        )
        .expect("check mailbox");

        let ledger = read_ledger(&path).expect("ledger");
        assert_eq!(ledger.cursor.last_seen_uid.as_deref(), Some("10"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn parser_accepts_action_commands_and_aliases() {
        assert_eq!(
            parse_reply_command("ACTION: NEXT"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Next,
                detail: None
            }
        );
        assert_eq!(
            parse_reply_command("@moss: continue"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Next,
                detail: None
            }
        );
        assert_eq!(
            parse_reply_command("@moss: stop"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Stop,
                detail: None
            }
        );
        assert_eq!(
            parse_reply_command("ACTION: CHANGE\nDETAIL: keep backend only"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Change,
                detail: Some("keep backend only".to_string())
            }
        );
        assert_eq!(
            parse_reply_command("继续"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Next,
                detail: None
            }
        );
        assert_eq!(
            parse_reply_command("状态"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Status,
                detail: None
            }
        );
        assert_eq!(
            parse_reply_command("你在干啥"),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Change,
                detail: Some("你在干啥".to_string())
            }
        );
        assert_eq!(
            parse_reply_command(
                "DETAIL: 你在干啥\n\n发件人：\"Most\" <moss@example.com>\n本轮已完成。"
            ),
            ParsedReplyCommand::Command {
                action: EmailMailCommandAction::Change,
                detail: Some("你在干啥".to_string())
            }
        );
    }

    #[test]
    fn parser_routes_invalid_or_risky_replies_to_confirmation() {
        assert_eq!(
            parse_reply_command("ACTION: CHANGE"),
            ParsedReplyCommand::NeedsConfirmation("missing_change_detail".to_string())
        );
        assert_eq!(
            parse_reply_command("ACTION: NEXT\nACTION: STOP"),
            ParsedReplyCommand::NeedsConfirmation("multiple_actions".to_string())
        );
        assert_eq!(
            parse_reply_command("ACTION: CHANGE\nDETAIL: run rm -rf /"),
            ParsedReplyCommand::NeedsConfirmation("high_risk_or_out_of_scope_detail".to_string())
        );
    }

    #[test]
    fn reply_slicing_removes_quoted_context_and_signature() {
        let sliced = strip_quoted_reply_text(
            "ACTION: CHANGE\nDETAIL: use smaller scope\n--- Reply above this line ---\n> ACTION: NEXT\n-- \nSig",
        );
        assert_eq!(sliced, "ACTION: CHANGE\nDETAIL: use smaller scope");
    }

    #[test]
    fn raw_email_parser_extracts_threading_headers_and_plain_body() {
        let raw = concat!(
            "Message-ID: <reply-1@example.com>\r\n",
            "From: User <user@example.com>\r\n",
            "Subject: Re: [Moss #ms_thread] Done\r\n",
            "In-Reply-To: <moss-out@example.com>\r\n",
            "References: <older@example.com> <moss-out@example.com>\r\n",
            "Auto-Submitted: no\r\n",
            "X-Moss-Session-Id: ms_thread\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "ACTION: NEXT\r\n"
        );
        let parsed = parse_raw_email_message(
            Some(42),
            raw.as_bytes(),
            Some("2026-05-21T10:00:00Z".to_string()),
        )
        .expect("parse raw message");

        assert_eq!(parsed.uid.as_deref(), Some("42"));
        assert_eq!(parsed.message_id.as_deref(), Some("<reply-1@example.com>"));
        assert_eq!(
            parsed.in_reply_to.as_deref(),
            Some("<moss-out@example.com>")
        );
        assert_eq!(
            parsed.references,
            vec![
                "<older@example.com>".to_string(),
                "<moss-out@example.com>".to_string(),
            ]
        );
        assert_eq!(parsed.text_body.trim(), "ACTION: NEXT");
        assert_eq!(
            parsed.headers.get("X-Moss-Session-Id").map(String::as_str),
            Some("ms_thread")
        );
    }

    #[test]
    fn mailbox_intake_accepts_valid_reply_and_deduplicates() {
        let path = temp_ledger_path("accept");
        let prepared = prepare_outgoing_continuation(
            &path,
            OutgoingContinuationRegistration {
                session_id: "ms_thread".to_string(),
                workspace_id: "ws-1".to_string(),
                workspace_name: Some("Workspace".to_string()),
                thread_id: "thread-1".to_string(),
                thread_name: Some("Thread".to_string()),
                turn_id: "turn-1".to_string(),
                subject: "Moss completed".to_string(),
                summary: "done".to_string(),
                next_recommendations: vec!["run tests".to_string()],
                actionable: true,
                action_window_hours: 24,
            },
            "本轮已完成\n--- Reply above this line ---",
        )
        .expect("prepare outgoing");
        let text_body = format!(
            "ACTION: NEXT\n{}\n",
            prepared
                .text_body
                .split(MOSS_CONTEXT_START)
                .nth(1)
                .map(|tail| format!("{MOSS_CONTEXT_START}{tail}"))
                .unwrap()
        );
        let message = EmailInboundMessage {
            uid: Some("1".to_string()),
            message_id: Some("<reply-1@example.com>".to_string()),
            from: "User <user@example.com>".to_string(),
            subject: format!("Re: {}", prepared.subject),
            text_body,
            in_reply_to: prepared.message_id.clone(),
            references: Vec::new(),
            headers: BTreeMap::new(),
            auto_submitted: Some("no".to_string()),
            received_at: None,
        };
        let result = check_mailbox_with_reader(
            &settings(),
            &path,
            &MemoryMailboxReader::new(vec![message.clone(), message]),
        )
        .expect("check mailbox");
        assert_eq!(result.queued_count, 1);
        assert_eq!(result.duplicate_count, 1);
        let ledger = read_ledger(&path).expect("ledger");
        assert_eq!(ledger.commands.len(), 2);
        assert_eq!(ledger.commands[0].detail.as_deref(), Some("run tests"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mailbox_intake_accepts_threaded_reply_when_webmail_strips_token_body() {
        let path = temp_ledger_path("stripped-token");
        let prepared = prepare_outgoing_continuation(
            &path,
            OutgoingContinuationRegistration {
                session_id: "ms_thread".to_string(),
                workspace_id: "ws-1".to_string(),
                workspace_name: Some("Workspace".to_string()),
                thread_id: "thread-1".to_string(),
                thread_name: Some("Thread".to_string()),
                turn_id: "turn-1".to_string(),
                subject: "Moss completed".to_string(),
                summary: "done".to_string(),
                next_recommendations: vec!["run tests".to_string()],
                actionable: true,
                action_window_hours: 24,
            },
            "本轮已完成\n--- Reply above this line ---",
        )
        .expect("prepare outgoing");
        let message = EmailInboundMessage {
            uid: Some("1".to_string()),
            message_id: Some("<reply-1@example.com>".to_string()),
            from: "User <user@example.com>".to_string(),
            subject: format!("Re: {}", prepared.subject),
            text_body: "状态".to_string(),
            in_reply_to: prepared.message_id.clone(),
            references: Vec::new(),
            headers: BTreeMap::new(),
            auto_submitted: Some("no".to_string()),
            received_at: None,
        };

        let result =
            check_mailbox_with_reader(&settings(), &path, &MemoryMailboxReader::new(vec![message]))
                .expect("check mailbox");

        assert_eq!(result.accepted_count, 1);
        assert_eq!(result.rejected_count, 0);
        let ledger = read_ledger(&path).expect("ledger");
        assert_eq!(ledger.commands.len(), 1);
        assert_eq!(ledger.commands[0].action, EmailMailCommandAction::Status);
        assert_eq!(ledger.commands[0].status, InboundCommandStatus::Done);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mailbox_intake_ignores_unrelated_mail_without_storage() {
        let path = temp_ledger_path("ignore");
        let message = EmailInboundMessage {
            uid: Some("1".to_string()),
            message_id: Some("<random@example.com>".to_string()),
            from: "other@example.com".to_string(),
            subject: "hello".to_string(),
            text_body: "not moss".to_string(),
            in_reply_to: None,
            references: Vec::new(),
            headers: BTreeMap::new(),
            auto_submitted: None,
            received_at: None,
        };
        let result =
            check_mailbox_with_reader(&settings(), &path, &MemoryMailboxReader::new(vec![message]))
                .expect("check mailbox");
        assert_eq!(result.ignored_count, 1);
        let ledger = read_ledger(&path).expect("ledger");
        assert!(ledger.commands.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mutate_delete_mail_records_removes_only_target_mail_records() {
        let path = temp_ledger_path("delete-mail-records");
        let ledger = EmailSessionLedger {
            outgoing: vec![
                OutgoingMailRecord {
                    id: "out-target".to_string(),
                    message_id: Some("<target@example.com>".to_string()),
                    session_id: "ms_target".to_string(),
                    workspace_id: "ws-1".to_string(),
                    workspace_name: Some("Workspace".to_string()),
                    thread_id: "thread-1".to_string(),
                    thread_name: Some("Thread".to_string()),
                    turn_id: "turn-1".to_string(),
                    reply_token_hash: Some("token".to_string()),
                    expires_at: None,
                    status: OutgoingMailStatus::Actionable,
                    sent_at: "2026-05-21T10:00:00Z".to_string(),
                    subject_tag: Some("Moss #ms_target".to_string()),
                    subject: "Target".to_string(),
                    summary: "target summary".to_string(),
                    next_recommendations: Vec::new(),
                    signature: None,
                },
                OutgoingMailRecord {
                    id: "out-other".to_string(),
                    message_id: Some("<other@example.com>".to_string()),
                    session_id: "ms_other".to_string(),
                    workspace_id: "ws-2".to_string(),
                    workspace_name: Some("Other Workspace".to_string()),
                    thread_id: "thread-2".to_string(),
                    thread_name: Some("Other Thread".to_string()),
                    turn_id: "turn-2".to_string(),
                    reply_token_hash: Some("other-token".to_string()),
                    expires_at: None,
                    status: OutgoingMailStatus::Actionable,
                    sent_at: "2026-05-21T11:00:00Z".to_string(),
                    subject_tag: Some("Moss #ms_other".to_string()),
                    subject: "Other".to_string(),
                    summary: "other summary".to_string(),
                    next_recommendations: Vec::new(),
                    signature: None,
                },
            ],
            commands: vec![
                InboundMailCommand {
                    id: "cmd-target".to_string(),
                    mail_message_id: "<reply-target@example.com>".to_string(),
                    in_reply_to: Some("<target@example.com>".to_string()),
                    linked_outgoing_mail_id: "out-target".to_string(),
                    session_id: "ms_target".to_string(),
                    workspace_id: "ws-1".to_string(),
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    reply_token_hash: "token".to_string(),
                    from_hash: "from-target".to_string(),
                    from_display: Some("u***@example.com".to_string()),
                    received_at: "2026-05-21T10:05:00Z".to_string(),
                    action: EmailMailCommandAction::Change,
                    detail: Some("target change".to_string()),
                    body_hash: "body-target".to_string(),
                    status: InboundCommandStatus::Queued,
                    reject_reason: None,
                    subject_tag: Some("Moss #ms_target".to_string()),
                },
                InboundMailCommand {
                    id: "cmd-other".to_string(),
                    mail_message_id: "<reply-other@example.com>".to_string(),
                    in_reply_to: Some("<other@example.com>".to_string()),
                    linked_outgoing_mail_id: "out-other".to_string(),
                    session_id: "ms_other".to_string(),
                    workspace_id: "ws-2".to_string(),
                    thread_id: "thread-2".to_string(),
                    turn_id: "turn-2".to_string(),
                    reply_token_hash: "other-token".to_string(),
                    from_hash: "from-other".to_string(),
                    from_display: Some("o***@example.com".to_string()),
                    received_at: "2026-05-21T11:05:00Z".to_string(),
                    action: EmailMailCommandAction::Status,
                    detail: None,
                    body_hash: "body-other".to_string(),
                    status: InboundCommandStatus::Done,
                    reject_reason: None,
                    subject_tag: Some("Moss #ms_other".to_string()),
                },
            ],
            sessions: vec![
                MailSessionControl {
                    session_id: "ms_target".to_string(),
                    state: MailSessionState::Paused,
                    mail_driven_enabled: true,
                    updated_at: "2026-05-21T10:00:00Z".to_string(),
                },
                MailSessionControl {
                    session_id: "ms_other".to_string(),
                    state: MailSessionState::Enabled,
                    mail_driven_enabled: true,
                    updated_at: "2026-05-21T11:00:00Z".to_string(),
                },
            ],
            cursor: EmailMailboxCursor::default(),
        };
        write_ledger(&path, &ledger).expect("write ledger");

        let projection = mutate_mail_session(
            &path,
            MutateMailSessionRequest {
                session_id: "ms_target".to_string(),
                action: "delete_mail_records".to_string(),
                command_id: None,
            },
        )
        .expect("delete target mail records");

        let next = read_ledger(&path).expect("read ledger");
        assert_eq!(next.outgoing.len(), 1);
        assert_eq!(next.outgoing[0].session_id, "ms_other");
        assert_eq!(next.commands.len(), 1);
        assert_eq!(next.commands[0].session_id, "ms_other");
        assert_eq!(next.sessions.len(), 2);
        assert!(next
            .sessions
            .iter()
            .any(|session| session.session_id == "ms_target"
                && session.state == MailSessionState::Paused));
        assert!(projection
            .sessions
            .iter()
            .all(|session| session.session_id != "ms_target"));
        assert!(projection
            .timeline
            .iter()
            .all(|event| event.session_id != "ms_target"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mutate_delete_mail_records_requires_session_id_without_writing() {
        let path = temp_ledger_path("delete-mail-records-missing-session");
        let ledger = EmailSessionLedger {
            sessions: vec![MailSessionControl {
                session_id: "ms_target".to_string(),
                state: MailSessionState::Enabled,
                mail_driven_enabled: true,
                updated_at: "2026-05-21T10:00:00Z".to_string(),
            }],
            ..EmailSessionLedger::default()
        };
        write_ledger(&path, &ledger).expect("write ledger");

        let error = mutate_mail_session(
            &path,
            MutateMailSessionRequest {
                session_id: " ".to_string(),
                action: "delete_mail_records".to_string(),
                command_id: None,
            },
        )
        .expect_err("missing session id should fail");

        assert!(error.contains("missing session id"));
        let next = read_ledger(&path).expect("read ledger");
        assert_eq!(next, ledger);
        let _ = std::fs::remove_file(path);
    }
}
