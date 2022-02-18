use std::{error, fmt};

#[derive(Debug, Clone)]
pub struct NoValidScreenResourceError;
impl fmt::Display for NoValidScreenResourceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not find valid screen resouce")
    }
}
impl error::Error for NoValidScreenResourceError {}

#[derive(Debug, Clone)]
pub struct NoValidBacklightRangeValuesError;
impl fmt::Display for NoValidBacklightRangeValuesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "did not receive proper backlight value range values")
    }
}
impl error::Error for NoValidBacklightRangeValuesError {}

#[derive(Debug, Clone)]
pub struct NoValidCurrenBacklightValueError;
impl fmt::Display for NoValidCurrenBacklightValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "did not receive proper current backlight value")
    }
}
impl error::Error for NoValidCurrenBacklightValueError {}

#[derive(Debug, Clone)]
pub struct ValueOutOfRangeError {
    pub min: u32,
    pub max: u32,
    pub val: u32,
}
impl fmt::Display for ValueOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "backlight value is out of range - min: {}, max: {}, value: {}",
            self.min, self.max, self.val
        )
    }
}
impl error::Error for ValueOutOfRangeError {}

#[derive(Debug, Clone)]
pub struct StepParameterOutOfRangeError {
    pub max: u32,
    pub step_val: u32,
}
impl fmt::Display for StepParameterOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "steps parameter is out of range - min: {}, max: {}, steps value: {}",
            0, self.max, self.step_val
        )
    }
}
impl error::Error for StepParameterOutOfRangeError {}
