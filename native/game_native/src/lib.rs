use rustler::NifResult;

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

rustler::init!("Elixir.Game.NifBridge");
