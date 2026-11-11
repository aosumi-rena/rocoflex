use super::{scalar, Cause, CoefficientError, Operation, Provider, ScalarUpper};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32InstructionSet, ScalarF32Opcode,
    ScalarF32Operation, ScalarF32Mxcsr};
use scalar::LogArithmeticError;

#[derive(Clone, Copy)]
enum Binary { Add, Subtract, Divide, Multiply }
impl<U: ScalarUpper> Provider<U> {
    pub fn addss_bits(&mut self, left: u32, right: u32) -> Result<u32, CoefficientError> {
        self.binary_value(Binary::Add, left, right)
    }
    pub fn subss_bits(&mut self, left: u32, right: u32) -> Result<u32, CoefficientError> {
        self.binary_value(Binary::Subtract, left, right)
    }

    pub fn signed_i32_to_f32_bits(&mut self, value: i32) -> Result<u32, CoefficientError> {
        let index = self.begin(Operation::ConvertI32 { value })?;
        let result = self.shared_once(ScalarF32Opcode::Cvtsi2ssSigned32,
            ScalarF32Operation::Cvtsi2ssSigned32 { value });
        self.finish(index, result)
    }
    pub fn divss_bits(&mut self, left: u32, right: u32) -> Result<u32, CoefficientError> {
        self.binary_value(Binary::Divide, left, right)
    }
    pub fn mulss_bits(&mut self, left: u32, right: u32) -> Result<u32, CoefficientError> {
        self.binary_value(Binary::Multiply, left, right)
    }
    fn binary_value(&mut self, binary: Binary, left: u32, right: u32) -> Result<u32, CoefficientError> {
        let operation = match binary {
            Binary::Add => Operation::Addss { left, right },
            Binary::Subtract => Operation::Subss { left, right },
            Binary::Divide => Operation::Divss { left, right },
            Binary::Multiply => Operation::Mulss { left, right },
        };
        let index = self.begin(operation)?;
        let result = self.binary_once(binary, left, right);
        self.finish(index, result)
    }
    fn binary_once(&mut self, binary: Binary, left: u32, right: u32) -> Result<u32, Cause<U::Error>> {
        let (opcode, operation) = match binary {
            Binary::Add => (ScalarF32Opcode::Addss, ScalarF32Operation::Addss { left, right }),
            Binary::Subtract => (ScalarF32Opcode::Subss, ScalarF32Operation::Subss { left, right }),
            Binary::Multiply => (ScalarF32Opcode::Mulss, ScalarF32Operation::Mulss { left, right }),
            Binary::Divide => return match scalar::raw_divss_with_explicit_state(left, right, self.state, self.config.architecture) {
                Ok(result) => { self.state = result.exception_state; Ok(result.bits) }
                Err(error) => {
                    self.state = match &error {
                        LogArithmeticError::ArithmeticRequired { advanced, .. }
                        | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                        | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                        | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => *advanced,
                        LogArithmeticError::Dispatch(_) => self.state,
                    };
                    Err(Cause::Divide(error))
                }
            },
        };
        self.shared_once(opcode, operation)
    }
    fn shared_once(&mut self, opcode: ScalarF32Opcode, operation: ScalarF32Operation) -> Result<u32, Cause<U::Error>> {

        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[opcode]).map_err(Cause::Shared)?;
        let mut backend = SourceScalarF32::from_explicit_architecture(self.config.architecture, instructions).map_err(Cause::Shared)?;
        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(Cause::ReportedState { mxcsr: self.state.mxcsr() })?;
        let outcome = backend.execute(operation, before).map_err(Cause::Shared)?;
        use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};
        match project_scalar_f32_outcome(self.state, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
            SourceScalarF32Projection::Trapped { reported_mxcsr: mxcsr, unmasked, advanced } => {
                self.state = advanced; Err(Cause::Trap { mxcsr, unmasked })
            }
            SourceScalarF32Projection::ReportedStateRequired { reported, known_advanced, .. } => {
                self.state = known_advanced; Err(Cause::ReportedScalarOutcome(reported))
            }
        }
    }
}
