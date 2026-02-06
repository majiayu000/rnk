//! Weather icons (Nerd Font)

/// Sunny/clear
pub const fn sunny() -> &'static str {
    ""
}

/// Cloudy
pub const fn cloudy() -> &'static str {
    ""
}

/// Partly cloudy (day)
pub const fn partly_cloudy_day() -> &'static str {
    ""
}

/// Partly cloudy (night)
pub const fn partly_cloudy_night() -> &'static str {
    ""
}

/// Rain
pub const fn rain() -> &'static str {
    ""
}

/// Heavy rain
pub const fn heavy_rain() -> &'static str {
    ""
}

/// Thunderstorm
pub const fn thunderstorm() -> &'static str {
    ""
}

/// Snow
pub const fn snow() -> &'static str {
    ""
}

/// Fog
pub const fn fog() -> &'static str {
    ""
}

/// Wind
pub const fn wind() -> &'static str {
    ""
}

/// Tornado
pub const fn tornado() -> &'static str {
    ""
}

/// Hurricane
pub const fn hurricane() -> &'static str {
    ""
}

/// Night/moon
pub const fn night() -> &'static str {
    ""
}

/// Sunrise
pub const fn sunrise() -> &'static str {
    ""
}

/// Sunset
pub const fn sunset() -> &'static str {
    ""
}

/// Thermometer
pub const fn thermometer() -> &'static str {
    ""
}

/// Humidity
pub const fn humidity() -> &'static str {
    ""
}

/// Barometer
pub const fn barometer() -> &'static str {
    ""
}

/// Umbrella
pub const fn umbrella() -> &'static str {
    ""
}

/// Snowflake
pub const fn snowflake() -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_icons() {
        assert_eq!(sunny(), "");
        assert_eq!(rain(), "");
        assert_eq!(snow(), "");
    }
}
