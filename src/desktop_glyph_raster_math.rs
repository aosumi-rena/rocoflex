use super::{raster, scalar, Cause, ConversionCompletion, Lowered, Provider, Reply, Stopped,
    TraceRequest, SourceF64ValueMath, SourcePcmProbeMath, SourceRawSquareRootF64};

impl Provider {
    pub(super) fn execute_raster(&mut self, index: usize, request: raster::Request,
        instruction: raster::Admitted) -> Result<Reply, Cause> {

        match instruction.primitive {
            raster::Primitive::CmpeqpsXmm | raster::Primitive::Maxss => {
                let lowered = raster::lower_packed_max_subset(request, instruction)
                    .map_err(Cause::RasterPackedMaxLowering)?;
                self.calls[index].lowered = Some(Lowered::PackedMax(lowered));
                self.execute_packed_max(index, lowered.operation())
            }
            raster::Primitive::Ucomisd | raster::Primitive::Sqrtsd => {
                let lowered = raster::lower_guard_fast_subset(request, instruction)
                    .map_err(Cause::RasterGuardFastLowering)?;
                self.calls[index].lowered = Some(Lowered::GuardFast(lowered));
                self.execute_guard_fast(index, lowered.operation())
            }
            raster::Primitive::Cvtsi2sdSigned64 | raster::Primitive::Cvttsd2si32
            | raster::Primitive::Cvttss2si64 | raster::Primitive::Cvttsd2si64 => {
                let lowered = raster::lower_conversion_subset(request, instruction)
                    .map_err(Cause::RasterConversionLowering)?;
                self.calls[index].lowered = Some(Lowered::Conversion(lowered));
                self.execute_conversion(index, lowered.operation())
            }
            raster::Primitive::Addsd | raster::Primitive::Mulsd
            | raster::Primitive::Cvtss2sd | raster::Primitive::Cvtsi2sdSigned32 => {
                let lowered = raster::lower_f64_subset(request, instruction).map_err(Cause::RasterF64Lowering)?;
                self.calls[index].lowered = Some(Lowered::F64(lowered));
                self.execute_wide(index, lowered.operation())
            }
            _ => {
                let lowered = raster::lower_f32_subset(request, instruction).map_err(Cause::RasterLowering)?;

                self.calls[index].lowered = Some(Lowered::F32(lowered));
                self.execute_low(index, lowered.operation())
            }
        }
    }

    fn execute_wide(&mut self, index: usize, operation: raster::F64LoweredOperation) -> Result<Reply, Cause> {
        let entry = self.state;
        let mut leaf = match SourceF64ValueMath::bind(self.architecture, entry) {
            Ok(leaf) => leaf,
            Err(error) => {
                self.state = error.known_advanced_state(entry);
                return Err(Cause::Wide(error));
            }
        };
        let result = match operation {
            raster::F64LoweredOperation::Addsd { left, right } => leaf.add_bits(left, right),
            raster::F64LoweredOperation::Mulsd { left, right } => leaf.multiply_bits(left, right),
            raster::F64LoweredOperation::Cvtss2sd { source } => leaf.widen_f32_bits(source),
            raster::F64LoweredOperation::Cvtsi2sdSigned32 { value } => leaf.signed_i32_to_f64_bits(value),
        };

        self.calls[index].wide_candidate = leaf.last_candidate();
        self.calls[index].wide_completion = leaf.last_completion();
        self.state = leaf.exception_state();
        match result {
            Ok(bits) => Ok(Reply::Bits64(bits)),
            Err(error) => {
                self.failed_wide = Some(leaf);
                Err(Cause::Wide(error))
            }
        }
    }

    fn bind_pcm_conversion(&mut self) -> Result<SourcePcmProbeMath, Cause> {
        let entry = self.state;
        SourcePcmProbeMath::bind(self.architecture, entry).map_err(|error| {
            self.state = error.known_advanced_state(entry);
            Cause::PcmConversion(error)
        })
    }

    fn execute_conversion(&mut self, index: usize, operation: raster::ConversionLoweredOperation)
        -> Result<Reply, Cause> {
        use raster::ConversionLoweredOperation as Conversion;
        let entry = self.state;
        match operation {
            Conversion::Cvtsi2sdSigned64 { value } => {
                let mut leaf = self.bind_pcm_conversion()?;
                let result = leaf.signed_i64_to_f64_bits_with_status(value);
                if let Ok(completion) = &result {
                    self.calls[index].conversion_completion = Some(ConversionCompletion::SignedI64ToF64(*completion));
                }
                self.state = leaf.exception_state();
                match result {
                    Ok(completion) => Ok(Reply::Bits64(completion.bits)),
                    Err(error) => {
                        self.failed_conversion = Some(leaf);
                        Err(Cause::PcmConversion(error))
                    }
                }
            }
            Conversion::Cvttsd2si32 { source } => {
                match scalar::truncate_f64_to_i32_with_explicit_state(source, entry, self.architecture) {
                    Ok(completion) => {
                        self.calls[index].conversion_completion = Some(ConversionCompletion::F64ToI32(completion));
                        self.state = completion.exception_state;
                        Ok(Reply::Integer32Bits(completion.low_dword))
                    }
                    Err(error) => {
                        self.state = error.known_advanced_state(entry);
                        Err(Cause::Truncation(error))
                    }
                }
            }
            Conversion::Cvttss2si64 { source } => {
                match scalar::truncate_f32_to_i64_with_explicit_state(source, entry, self.architecture) {
                    Ok(completion) => {
                        self.calls[index].conversion_completion = Some(ConversionCompletion::F32ToI64(completion));
                        self.state = completion.exception_state;
                        Ok(Reply::Integer64Bits(completion.low_qword))
                    }
                    Err(error) => {
                        self.state = error.known_advanced_state(entry);
                        Err(Cause::Truncation(error))
                    }
                }
            }
            Conversion::Cvttsd2si64 { source } => {
                let mut leaf = self.bind_pcm_conversion()?;
                let completion = leaf.truncate_f64_bits_to_i64_with_status(source);
                self.calls[index].conversion_completion = Some(ConversionCompletion::F64ToI64(completion));
                self.state = leaf.exception_state();

                Ok(Reply::Integer64Bits(completion.value as u64))
            }
        }
    }

    fn execute_guard_fast(&mut self, index: usize, operation: raster::GuardFastLoweredOperation)
        -> Result<Reply, Cause> {
        let entry = self.state;
        match operation {
            raster::GuardFastLoweredOperation::Ucomisd { left, right } => {
                match scalar::ucomisd_f64_with_explicit_state(left, right, entry, self.architecture) {
                    Ok(outcome) => {

                        self.calls[index].guard_outcome = Some(outcome);
                        self.state = outcome.exception_state;
                        match outcome.reported {
                            scalar::SourceRawUcomisdOutcome::Completed { flags, .. } => Ok(Reply::F64Flags(flags)),
                            scalar::SourceRawUcomisdOutcome::Trapped { .. } => Err(Cause::GuardTrap(outcome)),
                        }
                    }
                    Err(error) => {
                        self.state = error.known_advanced_state(entry);
                        Err(Cause::Guard(error))
                    }
                }
            }
            raster::GuardFastLoweredOperation::Sqrtsd { source } => {
                let mut leaf = match SourceRawSquareRootF64::bind(self.architecture, entry) {
                    Ok(leaf) => leaf,
                    Err(error) => {
                        self.state = error.known_advanced_state(entry);
                        return Err(Cause::SquareRoot(error));
                    }
                };
                let result = leaf.evaluate_bits_with_status(source);
                if let Ok(completion) = &result {
                    self.calls[index].sqrt_completion = Some(*completion);
                }
                self.state = leaf.exception_state();
                match result {
                    Ok(completion) => Ok(Reply::Bits64(completion.bits)),
                    Err(error) => {
                        self.failed_sqrt = Some(leaf);
                        Err(Cause::SquareRoot(error))
                    }
                }
            }
        }
    }

    fn execute_packed_max(&mut self, index: usize, operation: raster::PackedMaxLoweredOperation)
        -> Result<Reply, Cause> {
        let entry = self.state;
        let before = self.before()?;
        match operation {
            raster::PackedMaxLoweredOperation::CmpeqpsXmm { left, right } => {
                let reported = self.shared.execute_cmpeqps_xmm(left, right, before).map_err(Cause::Shared)?;

                self.calls[index].packed_outcome = Some(reported);
                use scalar::logical_f32_outcome::{project_xmm_f32_outcome, SourceXmmF32Projection};
                match project_xmm_f32_outcome(entry, reported) {
                    SourceXmmF32Projection::Completed { bits, advanced } => {
                        self.state = advanced;
                        Ok(Reply::MasksXmm(bits))
                    }
                    SourceXmmF32Projection::Trapped { advanced, .. } => {
                        self.state = advanced;
                        Err(Cause::PackedTrap(reported))
                    }
                    SourceXmmF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                        self.state = known_advanced;
                        Err(Cause::ReportedPackedOutcome { reported, reason })
                    }
                }
            }
            raster::PackedMaxLoweredOperation::Maxss { left, right } => {
                let reported = self.shared.execute_maxss(left, right, before).map_err(Cause::Shared)?;
                self.calls[index].selection_outcome = Some(reported);
                use scalar::logical_f32_outcome::{project_selection_f32_outcome, SourceSelectionF32Projection};
                match project_selection_f32_outcome(entry, reported) {
                    SourceSelectionF32Projection::Completed { bits, advanced } => {
                        self.state = advanced;
                        Ok(Reply::MaxBits(bits))
                    }
                    SourceSelectionF32Projection::Trapped { advanced, .. } => {
                        self.state = advanced;
                        Err(Cause::SelectionTrap(reported))
                    }
                    SourceSelectionF32Projection::Required { required, advanced } => {
                        self.state = advanced;
                        Err(Cause::SelectionRequired { reported, required })
                    }
                    SourceSelectionF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                        self.state = known_advanced;
                        Err(Cause::ReportedSelectionOutcome { reported, reason })
                    }
                }
            }
        }
    }
}

fn project_reply(lowered: Option<Lowered>, reported: Reply)
    -> Result<raster::Reply, Cause> {
    use raster::F32LoweredOperation as Low;
    let Some(descriptor) = lowered else {
        return Err(Cause::RasterInternalReply { lowered, reported });
    };
    match descriptor {
        Lowered::F32(descriptor) => match (descriptor.operation(), reported) {
            (Low::Addss { .. } | Low::Subss { .. } | Low::Mulss { .. } | Low::Divss { .. }, Reply::Bits(bits)) =>
                Ok(raster::Reply::Bits32(bits)),
            (Low::Cmpless { .. } | Low::Cmpeqss { .. }, Reply::Bits(bits)) => Ok(raster::Reply::Mask32(bits)),
            (Low::Ucomiss { .. }, Reply::Flags(flags)) =>
                Ok(raster::Reply::Flags { carry: flags.carry, parity: flags.parity, zero: flags.zero }),
            _ => Err(Cause::RasterInternalReply { lowered, reported }),
        },
        Lowered::F64(descriptor) => match (descriptor.operation(), reported) {
            (raster::F64LoweredOperation::Addsd { .. } | raster::F64LoweredOperation::Mulsd { .. }
                | raster::F64LoweredOperation::Cvtss2sd { .. } | raster::F64LoweredOperation::Cvtsi2sdSigned32 { .. },
                Reply::Bits64(bits)) => Ok(raster::Reply::Bits64(bits)),
            _ => Err(Cause::RasterInternalReply { lowered, reported }),
        },
        Lowered::Conversion(descriptor) => match (descriptor.operation(), reported) {
            (raster::ConversionLoweredOperation::Cvtsi2sdSigned64 { .. }, Reply::Bits64(bits)) =>
                Ok(raster::Reply::Bits64(bits)),
            (raster::ConversionLoweredOperation::Cvttsd2si32 { .. }, Reply::Integer32Bits(bits)) =>
                Ok(raster::Reply::Integer32(bits as i32)),
            (raster::ConversionLoweredOperation::Cvttss2si64 { .. }
                | raster::ConversionLoweredOperation::Cvttsd2si64 { .. }, Reply::Integer64Bits(bits)) =>
                Ok(raster::Reply::Integer64(bits as i64)),
            _ => Err(Cause::RasterInternalReply { lowered, reported }),
        },
        Lowered::GuardFast(descriptor) => match (descriptor.operation(), reported) {
            (raster::GuardFastLoweredOperation::Ucomisd { .. }, Reply::F64Flags(flags)) =>
                Ok(raster::Reply::Flags { carry: flags.carry, parity: flags.parity, zero: flags.zero }),
            (raster::GuardFastLoweredOperation::Sqrtsd { .. }, Reply::Bits64(bits)) =>
                Ok(raster::Reply::Bits64(bits)),
            _ => Err(Cause::RasterInternalReply { lowered, reported }),
        },
        Lowered::PackedMax(descriptor) => match (descriptor.operation(), reported) {
            (raster::PackedMaxLoweredOperation::CmpeqpsXmm { .. }, Reply::MasksXmm(bits)) =>
                Ok(raster::Reply::MasksXmm(bits)),
            (raster::PackedMaxLoweredOperation::Maxss { .. }, Reply::MaxBits(bits)) =>
                Ok(raster::Reply::Bits32(bits)),
            _ => Err(Cause::RasterInternalReply { lowered, reported }),
        },
    }
}

impl raster::NumericOwner for Provider {
    type Error = Stopped;
    fn execute(&mut self, request: raster::Request, instruction: raster::Admitted)
        -> Result<raster::Reply, Stopped> {
        self.call(TraceRequest::Raster { request, instruction }, project_reply)
    }
}
