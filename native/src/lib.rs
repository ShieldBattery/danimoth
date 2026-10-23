mod names;
mod sys;

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use neon::prelude::*;
use neon::types::Deferred;
use neon::types::function::TryIntoArguments;

use names::*;
use sys::*;

static SDK: OnceLock<Sdk> = OnceLock::new();

fn sdk(cx: &mut Cx) -> NeonResult<&'static Sdk> {
    match SDK.get() {
        Some(sdk) => Ok(sdk),
        None => cx.throw_error("The Discord Social SDK isn't loaded, call loadSdk first"),
    }
}

/// Returns the version of this module, so a caller can tell which build of it was loaded.
fn version(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string(env!("CARGO_PKG_VERSION")))
}

fn load_sdk(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let path = cx.argument::<JsString>(0)?.value(&mut cx);
    if SDK.get().is_none() {
        match unsafe { Sdk::load(Path::new(&path)) } {
            Ok(sdk) => {
                let _ = SDK.set(sdk);
            }
            Err(e) => {
                return cx.throw_error(format!(
                    "Failed to load the Discord Social SDK from {path}: {e}"
                ));
            }
        }
    }
    Ok(cx.undefined())
}

fn sdk_version(mut cx: FunctionContext) -> JsResult<JsObject> {
    let sdk = sdk(&mut cx)?;
    let (major, minor, patch, hash) = unsafe {
        (
            (sdk.Discord_Client_GetVersionMajor)(),
            (sdk.Discord_Client_GetVersionMinor)(),
            (sdk.Discord_Client_GetVersionPatch)(),
            sdk.get_string(|s| (sdk.Discord_Client_GetVersionHash)(s)),
        )
    };
    let version = cx.empty_object();
    version.prop(&mut cx, "major").set(major)?;
    version.prop(&mut cx, "minor").set(minor)?;
    version.prop(&mut cx, "patch").set(patch)?;
    version.prop(&mut cx, "hash").set(hash)?;
    Ok(version)
}

fn run_callbacks(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let sdk = sdk(&mut cx)?;
    unsafe { (sdk.Discord_RunCallbacks)() };
    Ok(cx.undefined())
}

/// A JS function the SDK calls back into. Calls are queued to the JS thread through `channel`, so
/// it doesn't matter which thread the SDK calls from.
struct JsCallback {
    channel: Channel,
    function: Arc<Root<JsFunction>>,
}

impl JsCallback {
    /// Wraps `function` and returns it as user data for an SDK callback setter, to be paired with
    /// `free_js_callback`.
    fn into_user_data(cx: &mut Cx, function: Handle<JsFunction>) -> *mut c_void {
        let mut channel = cx.channel();
        // The app keeps its client until it exits, so the callbacks must not hold the process open.
        channel.unref(cx);
        let callback = Self {
            channel,
            function: Arc::new(function.root(cx)),
        };
        Box::into_raw(Box::new(callback)) as *mut c_void
    }

    /// # Safety
    ///
    /// `user_data` must come from `into_user_data` and not have been freed.
    unsafe fn from_user_data<'a>(user_data: *mut c_void) -> &'a Self {
        unsafe { &*(user_data as *const Self) }
    }

    fn call<A>(&self, args: A)
    where
        A: for<'cx> TryIntoArguments<'cx> + Send + 'static,
    {
        let function = self.function.clone();
        // This only fails once the JS thread is shutting down, when there's no one left to call.
        let _ = self.channel.try_send(move |mut cx| {
            let function = function.to_inner(&mut cx);
            function.bind(&mut cx).args(args)?.exec()
        });
    }
}

unsafe extern "C" fn free_js_callback(user_data: *mut c_void) {
    drop(unsafe { Box::from_raw(user_data as *mut JsCallback) });
}

unsafe extern "C" fn log_callback(message: Discord_String, severity: i32, user_data: *mut c_void) {
    let Some(sdk) = SDK.get() else { return };
    let message = unsafe { sdk.take_string(message) };
    let callback = unsafe { JsCallback::from_user_data(user_data) };
    callback.call((message, log_severity_name(severity)));
}

unsafe extern "C" fn status_changed_callback(
    status: i32,
    error: i32,
    error_detail: i32,
    user_data: *mut c_void,
) {
    let callback = unsafe { JsCallback::from_user_data(user_data) };
    callback.call((
        client_status_name(status),
        client_error_name(error),
        error_detail,
    ));
}

unsafe extern "C" fn token_expiration_callback(user_data: *mut c_void) {
    let callback = unsafe { JsCallback::from_user_data(user_data) };
    // Neon can't call with an empty argument list here (`()` isn't `TryIntoArguments`), so this
    // passes a single `undefined`, which looks the same to a callback that takes no arguments.
    callback.call(((),));
}

unsafe extern "C" fn activity_join_callback(join_secret: Discord_String, user_data: *mut c_void) {
    let Some(sdk) = SDK.get() else { return };
    let join_secret = unsafe { sdk.take_string(join_secret) };
    let callback = unsafe { JsCallback::from_user_data(user_data) };
    callback.call((join_secret,));
}

/// Why an SDK call failed, copied out of its `Discord_ClientResult`.
struct CallError {
    message: String,
    error_type: &'static str,
    error_code: i32,
    retryable: bool,
    retry_after: f32,
}

impl CallError {
    fn to_js<'cx>(&self, cx: &mut Cx<'cx>) -> JsResult<'cx, JsError> {
        let error = cx.error(&self.message)?;
        error.prop(cx, "errorType").set(self.error_type)?;
        error.prop(cx, "errorCode").set(self.error_code)?;
        error.prop(cx, "retryable").set(self.retryable)?;
        error.prop(cx, "retryAfter").set(self.retry_after)?;
        Ok(error)
    }
}

/// Copies the outcome out of a result the SDK handed to a callback, and drops the result.
///
/// # Safety
///
/// `result` must be a result the SDK handed to a callback, which hasn't been dropped yet.
unsafe fn take_result(sdk: &Sdk, result: *mut Discord_ClientResult) -> Result<(), CallError> {
    // Callbacks own the result they're given: copy the handle and drop it through the copy.
    let mut result =
        unsafe { Owned::from_raw(std::ptr::read(result), sdk.Discord_ClientResult_Drop) };
    let ptr = result.as_ptr();
    unsafe {
        if (sdk.Discord_ClientResult_Successful)(ptr) {
            return Ok(());
        }
        let mut message = sdk.get_string(|s| (sdk.Discord_ClientResult_Error)(ptr, s));
        if message.is_empty() {
            message = sdk.get_string(|s| (sdk.Discord_ClientResult_ToString)(ptr, s));
        }
        Err(CallError {
            message,
            error_type: error_type_name((sdk.Discord_ClientResult_Type)(ptr)),
            error_code: (sdk.Discord_ClientResult_ErrorCode)(ptr),
            retryable: (sdk.Discord_ClientResult_Retryable)(ptr),
            retry_after: (sdk.Discord_ClientResult_RetryAfter)(ptr),
        })
    }
}

/// An SDK call whose completion settles a JS promise. Until then its channel holds the process open.
struct PendingCall {
    unsettled: Option<(Channel, Deferred)>,
}

impl PendingCall {
    /// Returns the promise for a call, and the user data to pass to the SDK along with
    /// `call_result_callback` and `free_pending_call`.
    fn start<'cx>(cx: &mut Cx<'cx>) -> (Handle<'cx, JsPromise>, *mut c_void) {
        let (deferred, promise) = cx.promise();
        let pending = Box::new(PendingCall {
            unsettled: Some((cx.channel(), deferred)),
        });
        (promise, Box::into_raw(pending) as *mut c_void)
    }

    fn settle(&mut self, outcome: Result<(), CallError>) {
        // The SDK frees the user data (and so the channel) only during a later `runCallbacks`, so
        // the channel is dropped here instead, to stop holding the process open once settled.
        let Some((channel, deferred)) = self.unsettled.take() else {
            return;
        };
        // This only fails once the JS thread is shutting down, when no one is awaiting the promise.
        let _ = deferred.try_settle_with(&channel, move |mut cx| match outcome {
            Ok(()) => Ok(cx.undefined()),
            Err(e) => {
                let error = e.to_js(&mut cx)?;
                cx.throw(error)
            }
        });
    }
}

unsafe extern "C" fn call_result_callback(
    result: *mut Discord_ClientResult,
    user_data: *mut c_void,
) {
    let Some(sdk) = SDK.get() else { return };
    let outcome = unsafe { take_result(sdk, result) };
    let pending = unsafe { &mut *(user_data as *mut PendingCall) };
    pending.settle(outcome);
}

unsafe extern "C" fn free_pending_call(user_data: *mut c_void) {
    let mut pending = unsafe { Box::from_raw(user_data as *mut PendingCall) };
    // The SDK frees a call's user data after running its callback, or without running it when the
    // client is dropped first. Only the latter leaves the promise unsettled.
    pending.settle(Err(CallError {
        message: "The Discord client was destroyed before the call completed".into(),
        error_type: error_type_name(Discord_ErrorType_ClientDestroyed),
        error_code: 0,
        retryable: false,
        retry_after: 0.0,
    }));
}

/// A Discord Social SDK client, handed to JS as an opaque handle.
struct Client {
    raw: RefCell<Option<Owned<Discord_Client>>>,
}

impl Finalize for Client {}

impl Client {
    /// Runs `f` with the SDK and this client's handle, throwing if the client was destroyed.
    ///
    /// The client stays borrowed while `f` runs, so `f` must not run JS (e.g. by reading properties
    /// of a JS object, which may be getters): JS could call back in with the same client.
    fn with<'cx, R>(
        &self,
        cx: &mut Cx<'cx>,
        f: impl FnOnce(&mut Cx<'cx>, &'static Sdk, *mut Discord_Client) -> NeonResult<R>,
    ) -> NeonResult<R> {
        let sdk = sdk(cx)?;
        let mut raw = self.raw.borrow_mut();
        let Some(raw) = raw.as_mut() else {
            return cx.throw_error("This Discord client was destroyed");
        };
        f(cx, sdk, raw.as_ptr())
    }
}

fn create_client(mut cx: FunctionContext) -> JsResult<JsBox<Client>> {
    let options = cx.argument::<JsObject>(0)?;
    let sdk = sdk(&mut cx)?;

    let on_log = options
        .prop(&mut cx, "onLog")
        .get::<Option<Handle<JsFunction>>>()?;
    let log_severity = match options
        .prop(&mut cx, "logSeverity")
        .get::<Option<String>>()?
    {
        Some(name) => parse_log_severity(&mut cx, &name)?,
        None => Discord_LoggingSeverity_Info,
    };
    let on_status_changed = options
        .prop(&mut cx, "onStatusChanged")
        .get::<Option<Handle<JsFunction>>>()?;
    let on_token_expiring = options
        .prop(&mut cx, "onTokenExpiring")
        .get::<Option<Handle<JsFunction>>>()?;
    let on_activity_join = options
        .prop(&mut cx, "onActivityJoin")
        .get::<Option<Handle<JsFunction>>>()?;

    let mut raw = unsafe { Owned::new(sdk.Discord_Client_Init, sdk.Discord_Client_Drop) };
    let ptr = raw.as_ptr();
    // The SDK recommends adding the log callback first, so it sees everything the client logs, and
    // the join callback must be set before connecting, so a join that launched the app isn't
    // missed.
    unsafe {
        if let Some(f) = on_log {
            let user_data = JsCallback::into_user_data(&mut cx, f);
            (sdk.Discord_Client_AddLogCallback)(
                ptr,
                log_callback,
                free_js_callback,
                user_data,
                log_severity,
            );
        }
        if let Some(f) = on_status_changed {
            let user_data = JsCallback::into_user_data(&mut cx, f);
            (sdk.Discord_Client_SetStatusChangedCallback)(
                ptr,
                status_changed_callback,
                free_js_callback,
                user_data,
            );
        }
        if let Some(f) = on_token_expiring {
            let user_data = JsCallback::into_user_data(&mut cx, f);
            (sdk.Discord_Client_SetTokenExpirationCallback)(
                ptr,
                token_expiration_callback,
                free_js_callback,
                user_data,
            );
        }
        if let Some(f) = on_activity_join {
            let user_data = JsCallback::into_user_data(&mut cx, f);
            (sdk.Discord_Client_SetActivityJoinCallback)(
                ptr,
                activity_join_callback,
                free_js_callback,
                user_data,
            );
        }
    }

    Ok(cx.boxed(Client {
        raw: RefCell::new(Some(raw)),
    }))
}

fn destroy_client(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    // Dropping the SDK client frees the JS callbacks it holds and rejects any pending calls.
    drop(client.raw.borrow_mut().take());
    Ok(cx.undefined())
}

fn parse_application_id(cx: &mut Cx, id: &str) -> NeonResult<u64> {
    match id.parse::<u64>() {
        Ok(id) => Ok(id),
        Err(_) => cx.throw_type_error(format!("Invalid Discord application ID: {id}")),
    }
}

fn set_application_id(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    let application_id = cx.argument::<JsString>(1)?.value(&mut cx);
    let application_id = parse_application_id(&mut cx, &application_id)?;
    client.with(&mut cx, |_, sdk, ptr| {
        unsafe { (sdk.Discord_Client_SetApplicationId)(ptr, application_id) };
        Ok(())
    })?;
    Ok(cx.undefined())
}

fn update_token(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    let token = cx.argument::<JsString>(1)?.value(&mut cx);
    client.with(&mut cx, |cx, sdk, ptr| {
        let (promise, user_data) = PendingCall::start(cx);
        unsafe {
            (sdk.Discord_Client_UpdateToken)(
                ptr,
                Discord_AuthorizationTokenType_Bearer,
                Discord_String::borrow(&token),
                call_result_callback,
                free_pending_call,
                user_data,
            )
        };
        Ok(promise)
    })
}

fn connect(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    client.with(&mut cx, |_, sdk, ptr| {
        unsafe { (sdk.Discord_Client_Connect)(ptr) };
        Ok(())
    })?;
    Ok(cx.undefined())
}

fn disconnect(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    client.with(&mut cx, |_, sdk, ptr| {
        unsafe { (sdk.Discord_Client_Disconnect)(ptr) };
        Ok(())
    })?;
    Ok(cx.undefined())
}

fn get_status(mut cx: FunctionContext) -> JsResult<JsString> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    let status = client.with(&mut cx, |_, sdk, ptr| {
        Ok(unsafe { (sdk.Discord_Client_GetStatus)(ptr) })
    })?;
    Ok(cx.string(client_status_name(status)))
}

/// Passes an optional string property of `object` to an SDK setter that takes an optional string.
fn set_optional_string<'cx, T>(
    cx: &mut Cx<'cx>,
    object: Handle<'cx, JsObject>,
    key: &str,
    target: *mut T,
    setter: unsafe extern "C" fn(*mut T, *mut Discord_String),
) -> NeonResult<()> {
    if let Some(value) = object.prop(cx, key).get::<Option<String>>()? {
        let mut value = Discord_String::borrow(&value);
        unsafe { setter(target, &mut value) };
    }
    Ok(())
}

fn get_timestamp<'cx>(
    cx: &mut Cx<'cx>,
    object: Handle<'cx, JsObject>,
    key: &str,
) -> NeonResult<Option<u64>> {
    match object.prop(cx, key).get::<Option<f64>>()? {
        Some(ms) if ms.is_finite() && ms >= 0.0 => Ok(Some(ms as u64)),
        Some(ms) => cx.throw_range_error(format!("Invalid timestamp for {key}: {ms}")),
        None => Ok(None),
    }
}

fn get_party_size<'cx>(
    cx: &mut Cx<'cx>,
    object: Handle<'cx, JsObject>,
    key: &str,
) -> NeonResult<i32> {
    let size = object.prop(cx, key).get::<f64>()?;
    if size.fract() != 0.0 || !(0.0..=i32::MAX as f64).contains(&size) {
        return cx.throw_range_error(format!("Invalid party size for {key}: {size}"));
    }
    Ok(size as i32)
}

/// Builds an SDK activity from a JS `Activity` (see `index.d.ts`). The SDK's setters copy what
/// they're given, so each nested object is dropped once it's set.
fn build_activity<'cx>(
    cx: &mut Cx<'cx>,
    sdk: &Sdk,
    object: Handle<'cx, JsObject>,
) -> NeonResult<Owned<Discord_Activity>> {
    let mut activity = unsafe { Owned::new(sdk.Discord_Activity_Init, sdk.Discord_Activity_Drop) };
    let ptr = activity.as_ptr();

    if let Some(name) = object.prop(cx, "type").get::<Option<String>>()? {
        let activity_type = parse_activity_type(cx, &name)?;
        unsafe { (sdk.Discord_Activity_SetType)(ptr, activity_type) };
    }
    if let Some(name) = object
        .prop(cx, "statusDisplayType")
        .get::<Option<String>>()?
    {
        let mut display_type = parse_status_display_type(cx, &name)?;
        unsafe { (sdk.Discord_Activity_SetStatusDisplayType)(ptr, &mut display_type) };
    }
    set_optional_string(cx, object, "details", ptr, sdk.Discord_Activity_SetDetails)?;
    set_optional_string(cx, object, "state", ptr, sdk.Discord_Activity_SetState)?;

    if let Some(timestamps) = object
        .prop(cx, "timestamps")
        .get::<Option<Handle<JsObject>>>()?
    {
        let mut native = unsafe {
            Owned::new(
                sdk.Discord_ActivityTimestamps_Init,
                sdk.Discord_ActivityTimestamps_Drop,
            )
        };
        if let Some(start) = get_timestamp(cx, timestamps, "start")? {
            unsafe { (sdk.Discord_ActivityTimestamps_SetStart)(native.as_ptr(), start) };
        }
        if let Some(end) = get_timestamp(cx, timestamps, "end")? {
            unsafe { (sdk.Discord_ActivityTimestamps_SetEnd)(native.as_ptr(), end) };
        }
        unsafe { (sdk.Discord_Activity_SetTimestamps)(ptr, native.as_ptr()) };
    }

    if let Some(assets) = object
        .prop(cx, "assets")
        .get::<Option<Handle<JsObject>>>()?
    {
        let mut native = unsafe {
            Owned::new(
                sdk.Discord_ActivityAssets_Init,
                sdk.Discord_ActivityAssets_Drop,
            )
        };
        let target = native.as_ptr();
        for (key, setter) in [
            ("largeImage", sdk.Discord_ActivityAssets_SetLargeImage),
            ("largeText", sdk.Discord_ActivityAssets_SetLargeText),
            ("smallImage", sdk.Discord_ActivityAssets_SetSmallImage),
            ("smallText", sdk.Discord_ActivityAssets_SetSmallText),
        ] {
            set_optional_string(cx, assets, key, target, setter)?;
        }
        unsafe { (sdk.Discord_Activity_SetAssets)(ptr, target) };
    }

    if let Some(party) = object.prop(cx, "party").get::<Option<Handle<JsObject>>>()? {
        let id = party.prop(cx, "id").get::<String>()?;
        let current_size = get_party_size(cx, party, "currentSize")?;
        let max_size = get_party_size(cx, party, "maxSize")?;
        let privacy = match party
            .prop(cx, "privacy")
            .get::<Option<String>>()?
            .as_deref()
        {
            None | Some("private") => Discord_ActivityPartyPrivacy_Private,
            Some("public") => Discord_ActivityPartyPrivacy_Public,
            Some(other) => return cx.throw_type_error(format!("Unknown party privacy: {other}")),
        };
        let mut native = unsafe {
            Owned::new(
                sdk.Discord_ActivityParty_Init,
                sdk.Discord_ActivityParty_Drop,
            )
        };
        let target = native.as_ptr();
        unsafe {
            (sdk.Discord_ActivityParty_SetId)(target, Discord_String::borrow(&id));
            (sdk.Discord_ActivityParty_SetCurrentSize)(target, current_size);
            (sdk.Discord_ActivityParty_SetMaxSize)(target, max_size);
            (sdk.Discord_ActivityParty_SetPrivacy)(target, privacy);
            (sdk.Discord_Activity_SetParty)(ptr, target);
        }
    }

    if let Some(join_secret) = object.prop(cx, "joinSecret").get::<Option<String>>()? {
        let mut native = unsafe {
            Owned::new(
                sdk.Discord_ActivitySecrets_Init,
                sdk.Discord_ActivitySecrets_Drop,
            )
        };
        let target = native.as_ptr();
        unsafe {
            (sdk.Discord_ActivitySecrets_SetJoin)(target, Discord_String::borrow(&join_secret));
            (sdk.Discord_Activity_SetSecrets)(ptr, target);
        }
    }

    if let Some(platforms) = object
        .prop(cx, "supportedPlatforms")
        .get::<Option<Handle<JsArray>>>()?
    {
        let mut mask = 0;
        for platform in platforms.to_vec(cx)? {
            let name = platform.downcast_or_throw::<JsString, _>(cx)?.value(cx);
            mask |= parse_platform(cx, &name)?;
        }
        unsafe { (sdk.Discord_Activity_SetSupportedPlatforms)(ptr, mask) };
    }

    if let Some(buttons) = object
        .prop(cx, "buttons")
        .get::<Option<Handle<JsArray>>>()?
    {
        for button in buttons.to_vec(cx)? {
            let button = button.downcast_or_throw::<JsObject, _>(cx)?;
            let label = button.prop(cx, "label").get::<String>()?;
            let url = button.prop(cx, "url").get::<String>()?;
            let mut native = unsafe {
                Owned::new(
                    sdk.Discord_ActivityButton_Init,
                    sdk.Discord_ActivityButton_Drop,
                )
            };
            let target = native.as_ptr();
            unsafe {
                (sdk.Discord_ActivityButton_SetLabel)(target, Discord_String::borrow(&label));
                (sdk.Discord_ActivityButton_SetUrl)(target, Discord_String::borrow(&url));
                (sdk.Discord_Activity_AddButton)(ptr, target);
            }
        }
    }

    Ok(activity)
}

fn update_rich_presence(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    let activity = cx.argument::<JsObject>(1)?;
    let sdk = sdk(&mut cx)?;
    let mut activity = build_activity(&mut cx, sdk, activity)?;
    client.with(&mut cx, |cx, sdk, ptr| {
        let (promise, user_data) = PendingCall::start(cx);
        unsafe {
            (sdk.Discord_Client_UpdateRichPresence)(
                ptr,
                activity.as_ptr(),
                call_result_callback,
                free_pending_call,
                user_data,
            )
        };
        Ok(promise)
    })
}

fn clear_rich_presence(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    client.with(&mut cx, |_, sdk, ptr| {
        unsafe { (sdk.Discord_Client_ClearRichPresence)(ptr) };
        Ok(())
    })?;
    Ok(cx.undefined())
}

fn register_launch_command(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let client = cx.argument::<JsBox<Client>>(0)?;
    let application_id = cx.argument::<JsString>(1)?.value(&mut cx);
    let application_id = parse_application_id(&mut cx, &application_id)?;
    let command = cx.argument::<JsString>(2)?.value(&mut cx);
    let registered = client.with(&mut cx, |_, sdk, ptr| {
        Ok(unsafe {
            (sdk.Discord_Client_RegisterLaunchCommand)(
                ptr,
                application_id,
                Discord_String::borrow(&command),
            )
        })
    })?;
    Ok(cx.boolean(registered))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("version", version)?;
    cx.export_function("loadSdk", load_sdk)?;
    cx.export_function("sdkVersion", sdk_version)?;
    cx.export_function("runCallbacks", run_callbacks)?;
    cx.export_function("createClient", create_client)?;
    cx.export_function("destroyClient", destroy_client)?;
    cx.export_function("setApplicationId", set_application_id)?;
    cx.export_function("updateToken", update_token)?;
    cx.export_function("connect", connect)?;
    cx.export_function("disconnect", disconnect)?;
    cx.export_function("getStatus", get_status)?;
    cx.export_function("updateRichPresence", update_rich_presence)?;
    cx.export_function("clearRichPresence", clear_rich_presence)?;
    cx.export_function("registerLaunchCommand", register_launch_command)?;
    Ok(())
}
