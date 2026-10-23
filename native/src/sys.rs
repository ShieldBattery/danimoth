//! The parts of the Discord Social SDK's C API (`cdiscord.h`) this module uses, loaded from the SDK
//! DLL at runtime so the module builds without the SDK present.
//!
//! The declarations are written by hand against SDK 1.10. They must match `cdiscord.h` exactly:
//! check every signature here against the header when moving to a new SDK version. C enums are
//! declared as `i32` rather than Rust enums, since a newer SDK may pass values this module doesn't
//! know about.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::ffi::c_void;
use std::path::Path;
use std::ptr;

use libloading::Library;
use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library as WindowsLibrary};

/// A string the SDK reads (when passed in) or allocates (when handed back). Strings the SDK hands
/// back must be released with `Discord_Free`.
#[repr(C)]
pub struct Discord_String {
    pub ptr: *mut u8,
    pub size: usize,
}

impl Discord_String {
    /// Borrows `s` for the duration of a call. The SDK copies strings it keeps.
    pub fn borrow(s: &str) -> Self {
        Self {
            ptr: s.as_ptr() as *mut u8,
            size: s.len(),
        }
    }

    pub fn empty() -> Self {
        Self {
            ptr: ptr::null_mut(),
            size: 0,
        }
    }
}

macro_rules! handles {
    ($($name:ident),* $(,)?) => {$(
        /// An SDK object handle. Objects this module creates are wrapped in `Owned`.
        #[repr(C)]
        pub struct $name {
            opaque: *mut c_void,
        }

        impl Default for $name {
            fn default() -> Self {
                Self { opaque: ptr::null_mut() }
            }
        }
    )*};
}

handles!(
    Discord_Client,
    Discord_ClientResult,
    Discord_Activity,
    Discord_ActivityAssets,
    Discord_ActivityTimestamps,
    Discord_ActivityParty,
    Discord_ActivitySecrets,
    Discord_ActivityButton,
);

/// An SDK object this module created, dropped through the SDK when it goes out of scope.
///
/// The handle is a pointer to SDK-owned memory, so moving it is fine.
pub struct Owned<T> {
    raw: T,
    drop: unsafe extern "C" fn(*mut T),
}

impl<T: Default> Owned<T> {
    /// # Safety
    ///
    /// `init` and `drop` must be the SDK's init and drop functions for `T`.
    pub unsafe fn new(
        init: unsafe extern "C" fn(*mut T),
        drop: unsafe extern "C" fn(*mut T),
    ) -> Self {
        let mut raw = T::default();
        unsafe { init(&mut raw) };
        Self { raw, drop }
    }

    /// # Safety
    ///
    /// `raw` must be an SDK object whose ownership the SDK handed to the caller, and `drop` the
    /// SDK's drop function for `T`.
    pub unsafe fn from_raw(raw: T, drop: unsafe extern "C" fn(*mut T)) -> Self {
        Self { raw, drop }
    }

    pub fn as_ptr(&mut self) -> *mut T {
        &mut self.raw
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        unsafe { (self.drop)(&mut self.raw) }
    }
}

pub type Discord_FreeFn = unsafe extern "C" fn(ptr: *mut c_void);

pub type Discord_ActivityPartyPrivacy = i32;
pub const Discord_ActivityPartyPrivacy_Private: i32 = 0;
pub const Discord_ActivityPartyPrivacy_Public: i32 = 1;

pub type Discord_ActivityTypes = i32;
pub const Discord_ActivityTypes_Playing: i32 = 0;
pub const Discord_ActivityTypes_Streaming: i32 = 1;
pub const Discord_ActivityTypes_Listening: i32 = 2;
pub const Discord_ActivityTypes_Watching: i32 = 3;
pub const Discord_ActivityTypes_Competing: i32 = 5;

pub type Discord_StatusDisplayTypes = i32;
pub const Discord_StatusDisplayTypes_Name: i32 = 0;
pub const Discord_StatusDisplayTypes_State: i32 = 1;
pub const Discord_StatusDisplayTypes_Details: i32 = 2;

/// A bit set of `Discord_ActivityGamePlatforms_*` values.
pub type Discord_ActivityGamePlatforms = i32;
pub const Discord_ActivityGamePlatforms_Desktop: i32 = 1;
pub const Discord_ActivityGamePlatforms_Xbox: i32 = 2;
pub const Discord_ActivityGamePlatforms_Samsung: i32 = 4;
pub const Discord_ActivityGamePlatforms_IOS: i32 = 8;
pub const Discord_ActivityGamePlatforms_Android: i32 = 16;
pub const Discord_ActivityGamePlatforms_Embedded: i32 = 32;
pub const Discord_ActivityGamePlatforms_PS4: i32 = 64;
pub const Discord_ActivityGamePlatforms_PS5: i32 = 128;

pub type Discord_ErrorType = i32;
pub const Discord_ErrorType_None: i32 = 0;
pub const Discord_ErrorType_NetworkError: i32 = 1;
pub const Discord_ErrorType_HTTPError: i32 = 2;
pub const Discord_ErrorType_ClientNotReady: i32 = 3;
pub const Discord_ErrorType_Disabled: i32 = 4;
pub const Discord_ErrorType_ClientDestroyed: i32 = 5;
pub const Discord_ErrorType_ValidationError: i32 = 6;
pub const Discord_ErrorType_Aborted: i32 = 7;
pub const Discord_ErrorType_AuthorizationFailed: i32 = 8;
pub const Discord_ErrorType_RPCError: i32 = 9;

pub type Discord_Client_Error = i32;
pub const Discord_Client_Error_None: i32 = 0;
pub const Discord_Client_Error_ConnectionFailed: i32 = 1;
pub const Discord_Client_Error_UnexpectedClose: i32 = 2;
pub const Discord_Client_Error_ConnectionCanceled: i32 = 3;

pub type Discord_Client_Status = i32;
pub const Discord_Client_Status_Disconnected: i32 = 0;
pub const Discord_Client_Status_Connecting: i32 = 1;
pub const Discord_Client_Status_Connected: i32 = 2;
pub const Discord_Client_Status_Ready: i32 = 3;
pub const Discord_Client_Status_Reconnecting: i32 = 4;
pub const Discord_Client_Status_Disconnecting: i32 = 5;
pub const Discord_Client_Status_HttpWait: i32 = 6;

pub type Discord_AuthorizationTokenType = i32;
pub const Discord_AuthorizationTokenType_Bearer: i32 = 1;

pub type Discord_LoggingSeverity = i32;
pub const Discord_LoggingSeverity_Verbose: i32 = 1;
pub const Discord_LoggingSeverity_Info: i32 = 2;
pub const Discord_LoggingSeverity_Warning: i32 = 3;
pub const Discord_LoggingSeverity_Error: i32 = 4;

// Callbacks that hand over a `Discord_ClientResult*` give ownership of it to the callee, which must
// drop it. Strings they hand over must be released with `Discord_Free`.
pub type Discord_Client_LogCallback = unsafe extern "C" fn(
    message: Discord_String,
    severity: Discord_LoggingSeverity,
    user_data: *mut c_void,
);
pub type Discord_Client_OnStatusChanged = unsafe extern "C" fn(
    status: Discord_Client_Status,
    error: Discord_Client_Error,
    error_detail: i32,
    user_data: *mut c_void,
);
pub type Discord_Client_TokenExpirationCallback = unsafe extern "C" fn(user_data: *mut c_void);
pub type Discord_Client_ActivityJoinCallback =
    unsafe extern "C" fn(join_secret: Discord_String, user_data: *mut c_void);
/// The shape shared by `Discord_Client_UpdateTokenCallback` and
/// `Discord_Client_UpdateRichPresenceCallback`.
pub type Discord_Client_ResultCallback =
    unsafe extern "C" fn(result: *mut Discord_ClientResult, user_data: *mut c_void);

macro_rules! sdk_functions {
    ($($name:ident: fn($($arg:ty),* $(,)?) $(-> $ret:ty)?;)*) => {
        /// The loaded SDK DLL and the functions this module calls in it.
        pub struct Sdk {
            _library: Library,
            $(pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Sdk {
            /// Loads the SDK DLL at `path` and looks up every function this module uses, so a DLL
            /// that lacks one fails here rather than on first use.
            ///
            /// # Safety
            ///
            /// Loading a DLL runs its initialization code. `path` must be a Discord Social SDK DLL
            /// whose C API matches the declarations in this file.
            pub unsafe fn load(path: &Path) -> Result<Self, libloading::Error> {
                // LOAD_WITH_ALTERED_SEARCH_PATH resolves the DLL's own dependencies from its
                // directory first, rather than from the directory of the executable.
                let library: Library = unsafe {
                    WindowsLibrary::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH)?
                }
                .into();
                Ok(Self {
                    $($name: unsafe {
                        *library.get(concat!(stringify!($name), "\0").as_bytes())?
                    },)*
                    _library: library,
                })
            }
        }
    };
}

sdk_functions! {
    Discord_Free: fn(*mut c_void);
    Discord_RunCallbacks: fn();

    Discord_ClientResult_Drop: fn(*mut Discord_ClientResult);
    Discord_ClientResult_ToString: fn(*mut Discord_ClientResult, *mut Discord_String);
    Discord_ClientResult_Type: fn(*mut Discord_ClientResult) -> Discord_ErrorType;
    Discord_ClientResult_Error: fn(*mut Discord_ClientResult, *mut Discord_String);
    Discord_ClientResult_ErrorCode: fn(*mut Discord_ClientResult) -> i32;
    Discord_ClientResult_Successful: fn(*mut Discord_ClientResult) -> bool;
    Discord_ClientResult_Retryable: fn(*mut Discord_ClientResult) -> bool;
    Discord_ClientResult_RetryAfter: fn(*mut Discord_ClientResult) -> f32;

    Discord_ActivityAssets_Init: fn(*mut Discord_ActivityAssets);
    Discord_ActivityAssets_Drop: fn(*mut Discord_ActivityAssets);
    Discord_ActivityAssets_SetLargeImage: fn(*mut Discord_ActivityAssets, *mut Discord_String);
    Discord_ActivityAssets_SetLargeText: fn(*mut Discord_ActivityAssets, *mut Discord_String);
    Discord_ActivityAssets_SetSmallImage: fn(*mut Discord_ActivityAssets, *mut Discord_String);
    Discord_ActivityAssets_SetSmallText: fn(*mut Discord_ActivityAssets, *mut Discord_String);

    Discord_ActivityTimestamps_Init: fn(*mut Discord_ActivityTimestamps);
    Discord_ActivityTimestamps_Drop: fn(*mut Discord_ActivityTimestamps);
    Discord_ActivityTimestamps_SetStart: fn(*mut Discord_ActivityTimestamps, u64);
    Discord_ActivityTimestamps_SetEnd: fn(*mut Discord_ActivityTimestamps, u64);

    Discord_ActivityParty_Init: fn(*mut Discord_ActivityParty);
    Discord_ActivityParty_Drop: fn(*mut Discord_ActivityParty);
    Discord_ActivityParty_SetId: fn(*mut Discord_ActivityParty, Discord_String);
    Discord_ActivityParty_SetCurrentSize: fn(*mut Discord_ActivityParty, i32);
    Discord_ActivityParty_SetMaxSize: fn(*mut Discord_ActivityParty, i32);
    Discord_ActivityParty_SetPrivacy: fn(*mut Discord_ActivityParty, Discord_ActivityPartyPrivacy);

    Discord_ActivitySecrets_Init: fn(*mut Discord_ActivitySecrets);
    Discord_ActivitySecrets_Drop: fn(*mut Discord_ActivitySecrets);
    Discord_ActivitySecrets_SetJoin: fn(*mut Discord_ActivitySecrets, Discord_String);

    Discord_ActivityButton_Init: fn(*mut Discord_ActivityButton);
    Discord_ActivityButton_Drop: fn(*mut Discord_ActivityButton);
    Discord_ActivityButton_SetLabel: fn(*mut Discord_ActivityButton, Discord_String);
    Discord_ActivityButton_SetUrl: fn(*mut Discord_ActivityButton, Discord_String);

    Discord_Activity_Init: fn(*mut Discord_Activity);
    Discord_Activity_Drop: fn(*mut Discord_Activity);
    Discord_Activity_AddButton: fn(*mut Discord_Activity, *const Discord_ActivityButton);
    Discord_Activity_SetType: fn(*mut Discord_Activity, Discord_ActivityTypes);
    Discord_Activity_SetStatusDisplayType: fn(*mut Discord_Activity, *mut Discord_StatusDisplayTypes);
    Discord_Activity_SetState: fn(*mut Discord_Activity, *mut Discord_String);
    Discord_Activity_SetDetails: fn(*mut Discord_Activity, *mut Discord_String);
    Discord_Activity_SetAssets: fn(*mut Discord_Activity, *mut Discord_ActivityAssets);
    Discord_Activity_SetTimestamps: fn(*mut Discord_Activity, *mut Discord_ActivityTimestamps);
    Discord_Activity_SetParty: fn(*mut Discord_Activity, *mut Discord_ActivityParty);
    Discord_Activity_SetSecrets: fn(*mut Discord_Activity, *mut Discord_ActivitySecrets);
    Discord_Activity_SetSupportedPlatforms: fn(*mut Discord_Activity, Discord_ActivityGamePlatforms);

    Discord_Client_Init: fn(*mut Discord_Client);
    Discord_Client_Drop: fn(*mut Discord_Client);
    Discord_Client_GetVersionMajor: fn() -> i32;
    Discord_Client_GetVersionMinor: fn() -> i32;
    Discord_Client_GetVersionPatch: fn() -> i32;
    Discord_Client_GetVersionHash: fn(*mut Discord_String);
    Discord_Client_SetApplicationId: fn(*mut Discord_Client, u64);
    Discord_Client_AddLogCallback: fn(
        *mut Discord_Client,
        Discord_Client_LogCallback,
        Discord_FreeFn,
        *mut c_void,
        Discord_LoggingSeverity,
    );
    Discord_Client_SetStatusChangedCallback: fn(
        *mut Discord_Client,
        Discord_Client_OnStatusChanged,
        Discord_FreeFn,
        *mut c_void,
    );
    Discord_Client_SetTokenExpirationCallback: fn(
        *mut Discord_Client,
        Discord_Client_TokenExpirationCallback,
        Discord_FreeFn,
        *mut c_void,
    );
    Discord_Client_SetActivityJoinCallback: fn(
        *mut Discord_Client,
        Discord_Client_ActivityJoinCallback,
        Discord_FreeFn,
        *mut c_void,
    );
    Discord_Client_UpdateToken: fn(
        *mut Discord_Client,
        Discord_AuthorizationTokenType,
        Discord_String,
        Discord_Client_ResultCallback,
        Discord_FreeFn,
        *mut c_void,
    );
    Discord_Client_Connect: fn(*mut Discord_Client);
    Discord_Client_Disconnect: fn(*mut Discord_Client);
    Discord_Client_GetStatus: fn(*mut Discord_Client) -> Discord_Client_Status;
    Discord_Client_UpdateRichPresence: fn(
        *mut Discord_Client,
        *mut Discord_Activity,
        Discord_Client_ResultCallback,
        Discord_FreeFn,
        *mut c_void,
    );
    Discord_Client_ClearRichPresence: fn(*mut Discord_Client);
    Discord_Client_RegisterLaunchCommand: fn(*mut Discord_Client, u64, Discord_String) -> bool;
}

impl Sdk {
    /// Copies a string the SDK handed over and releases the SDK's copy.
    ///
    /// # Safety
    ///
    /// `s` must be a string the SDK allocated and handed to the caller, not yet released.
    pub unsafe fn take_string(&self, s: Discord_String) -> String {
        if s.ptr.is_null() {
            return String::new();
        }
        let value = String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(s.ptr, s.size) })
            .into_owned();
        unsafe { (self.Discord_Free)(s.ptr as *mut c_void) };
        value
    }

    /// Calls an SDK function that writes a newly allocated string to an out-parameter, and returns
    /// a copy of it.
    ///
    /// # Safety
    ///
    /// `f` must write a string the SDK allocated (or leave it empty).
    pub unsafe fn get_string(&self, f: impl FnOnce(*mut Discord_String)) -> String {
        let mut s = Discord_String::empty();
        f(&mut s);
        unsafe { self.take_string(s) }
    }
}
