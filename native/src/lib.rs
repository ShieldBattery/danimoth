use neon::prelude::*;

/// Returns the version of this module, so a caller can tell which build of it was loaded.
fn version(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string(env!("CARGO_PKG_VERSION")))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("version", version)?;
    Ok(())
}
