use super::FromLocusValue;

impl FromLocusValue for String {
    fn from_locus_value(value: &str) -> Result<Self, String> {
        Ok(scalar_value(value).trim_matches('"').to_owned())
    }
}

impl FromLocusValue for bool {
    fn from_locus_value(value: &str) -> Result<Self, String> {
        match scalar_value(value) {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            value => Err(format!("invalid bool value: {value}")),
        }
    }
}

macro_rules! impl_from_str_locus_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromLocusValue for $ty {
                fn from_locus_value(value: &str) -> Result<Self, String> {
                    scalar_value(value)
                        .parse()
                        .map_err(|error| format!("failed to parse locusfs value: {error}"))
                }
            }
        )*
    };
}

impl_from_str_locus_value!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

fn scalar_value(value: &str) -> &str {
    let value = value.trim();
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(kind), Some(separator))
            if kind.is_ascii_alphabetic() && separator.is_whitespace() =>
        {
            chars.as_str().trim()
        }
        _ => value,
    }
}
