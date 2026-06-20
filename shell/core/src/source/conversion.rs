use super::FromLocusValue;

impl FromLocusValue for String {
    fn from_locus_value(value: &str) -> Result<Self, String> {
        Ok(value.to_owned())
    }
}

impl FromLocusValue for bool {
    fn from_locus_value(value: &str) -> Result<Self, String> {
        match value.trim() {
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
                    value
                        .trim()
                        .parse()
                        .map_err(|error| format!("failed to parse locusfs value: {error}"))
                }
            }
        )*
    };
}

impl_from_str_locus_value!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);
