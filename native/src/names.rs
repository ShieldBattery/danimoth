//! Conversions between the SDK's enum values and the names `index.d.ts` uses for them.
// The SDK's constants keep their C names (see `sys.rs`).
#![allow(non_upper_case_globals)]

use neon::prelude::*;

use crate::sys::*;

pub fn client_status_name(status: Discord_Client_Status) -> &'static str {
    match status {
        Discord_Client_Status_Disconnected => "disconnected",
        Discord_Client_Status_Connecting => "connecting",
        Discord_Client_Status_Connected => "connected",
        Discord_Client_Status_Ready => "ready",
        Discord_Client_Status_Reconnecting => "reconnecting",
        Discord_Client_Status_Disconnecting => "disconnecting",
        Discord_Client_Status_HttpWait => "httpWait",
        _ => "unknown",
    }
}

pub fn client_error_name(error: Discord_Client_Error) -> &'static str {
    match error {
        Discord_Client_Error_None => "none",
        Discord_Client_Error_ConnectionFailed => "connectionFailed",
        Discord_Client_Error_UnexpectedClose => "unexpectedClose",
        Discord_Client_Error_ConnectionCanceled => "connectionCanceled",
        _ => "unknown",
    }
}

pub fn error_type_name(error_type: Discord_ErrorType) -> &'static str {
    match error_type {
        Discord_ErrorType_None => "none",
        Discord_ErrorType_NetworkError => "networkError",
        Discord_ErrorType_HTTPError => "httpError",
        Discord_ErrorType_ClientNotReady => "clientNotReady",
        Discord_ErrorType_Disabled => "disabled",
        Discord_ErrorType_ClientDestroyed => "clientDestroyed",
        Discord_ErrorType_ValidationError => "validationError",
        Discord_ErrorType_Aborted => "aborted",
        Discord_ErrorType_AuthorizationFailed => "authorizationFailed",
        Discord_ErrorType_RPCError => "rpcError",
        _ => "unknown",
    }
}

pub fn log_severity_name(severity: Discord_LoggingSeverity) -> &'static str {
    match severity {
        Discord_LoggingSeverity_Verbose => "verbose",
        Discord_LoggingSeverity_Info => "info",
        Discord_LoggingSeverity_Warning => "warning",
        Discord_LoggingSeverity_Error => "error",
        _ => "unknown",
    }
}

pub fn parse_log_severity(cx: &mut Cx, name: &str) -> NeonResult<Discord_LoggingSeverity> {
    match name {
        "verbose" => Ok(Discord_LoggingSeverity_Verbose),
        "info" => Ok(Discord_LoggingSeverity_Info),
        "warning" => Ok(Discord_LoggingSeverity_Warning),
        "error" => Ok(Discord_LoggingSeverity_Error),
        _ => cx.throw_type_error(format!("Unknown log severity: {name}")),
    }
}

pub fn parse_activity_type(cx: &mut Cx, name: &str) -> NeonResult<Discord_ActivityTypes> {
    match name {
        "playing" => Ok(Discord_ActivityTypes_Playing),
        "streaming" => Ok(Discord_ActivityTypes_Streaming),
        "listening" => Ok(Discord_ActivityTypes_Listening),
        "watching" => Ok(Discord_ActivityTypes_Watching),
        "competing" => Ok(Discord_ActivityTypes_Competing),
        _ => cx.throw_type_error(format!("Unknown activity type: {name}")),
    }
}

pub fn parse_status_display_type(
    cx: &mut Cx,
    name: &str,
) -> NeonResult<Discord_StatusDisplayTypes> {
    match name {
        "name" => Ok(Discord_StatusDisplayTypes_Name),
        "state" => Ok(Discord_StatusDisplayTypes_State),
        "details" => Ok(Discord_StatusDisplayTypes_Details),
        _ => cx.throw_type_error(format!("Unknown status display type: {name}")),
    }
}

pub fn parse_platform(cx: &mut Cx, name: &str) -> NeonResult<Discord_ActivityGamePlatforms> {
    match name {
        "desktop" => Ok(Discord_ActivityGamePlatforms_Desktop),
        "xbox" => Ok(Discord_ActivityGamePlatforms_Xbox),
        "samsung" => Ok(Discord_ActivityGamePlatforms_Samsung),
        "ios" => Ok(Discord_ActivityGamePlatforms_IOS),
        "android" => Ok(Discord_ActivityGamePlatforms_Android),
        "embedded" => Ok(Discord_ActivityGamePlatforms_Embedded),
        "ps4" => Ok(Discord_ActivityGamePlatforms_PS4),
        "ps5" => Ok(Discord_ActivityGamePlatforms_PS5),
        _ => cx.throw_type_error(format!("Unknown platform: {name}")),
    }
}
