use crate::MAX_REFERENCE_DEPTH;

/// Bound parser recursion before handing untrusted input to Fluent's recursive parser.
/// Quoted braces and comment lines cannot increase expression depth.
pub(crate) fn validate_source(source: &str) -> Result<(), String> {
    let mut braces = 0usize;
    let mut calls = 0usize;
    for line in source.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut quoted = false;
        let mut escaped = false;
        for byte in line.bytes() {
            if byte == 0 {
                return Err("NUL is not allowed in Fluent resources".into());
            }
            if quoted {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = false;
                }
                continue;
            }
            match byte {
                b'"' if braces > 0 => quoted = true,
                b'{' => braces += 1,
                b'}' => {
                    braces = braces.saturating_sub(1);
                    if braces == 0 {
                        calls = 0;
                    }
                }
                b'(' if braces > 0 => calls += 1,
                b')' if braces > 0 => calls = calls.saturating_sub(1),
                _ => {}
            }
            if braces + calls > MAX_REFERENCE_DEPTH {
                return Err("Fluent expression nesting exceeds the limit".into());
            }
        }
    }
    Ok(())
}
