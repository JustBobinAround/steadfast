mod record_index;
mod record_ledger;
#[macro_export]
macro_rules! lock_shared {
    ($to_lock: expr => $block:block) => {{
        $to_lock.lock_shared().map_err(|_| ())?;
        let res = $block;
        $to_lock.unlock().map_err(|_| ())?;
        res
    }};
}

/// Helper macro that will release file lock before handling inner result
#[macro_export]
macro_rules! lock {
    ($to_lock: expr => $block:block) => {{
        $to_lock.lock().map_err(|_| ())?;
        let res = $block;
        $to_lock.unlock().map_err(|_| ())?;
        res
    }};
}
#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    // }
}
