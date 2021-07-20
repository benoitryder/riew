
/// Return early on None
macro_rules! try_some {
    ($e:expr, $d:expr) => {
        match $e {
            Some(v) => v,
            None => return $d,
        }
    };
    ($e:expr) => { try_some!($e, ()) }
}

/// Cast an `(x, y)` tuple
macro_rules! size_as {
    ($e:expr, $t:ty) => {{
        let (x, y) = $e;
        (x as $t, y as $t)
    }}
}

