#[path = "../support/positive_log.rs"]
mod cpu0_log;
mod fused_log;
mod separate_reduction;
mod separate_large_reduction;
#[path = "../../numeric/explicit_scalar_f32.rs"]
pub mod shared_scalar_f32;
#[path = "../support/power.rs"]
mod retained_power;
#[path = "../../dsp/generated_coefficient_exp_constants.rs"]
mod retained_exp_constants;
#[path = "../../dsp/generated_scalar_large_constants.rs"]
mod retained_phase_constants;

pub mod finite_power;
mod phase_status;
mod integer_truncate;
pub mod pcm_probe_math;
pub mod f64_value_math;
pub mod scalar_truncate;
pub mod raw_scalar_truncate;
pub mod raw_integer_to_f64;
pub mod raw_integer_to_f32;
pub mod raw_ucomiss;
pub mod raw_scalar_maximum_f32;
pub mod raw_scalar_minimum_f32;
mod scalar_compare_f64;
pub mod raw_ucomisd;
pub mod raw_scalar_select_f64;
pub mod packed_compare_f64;
mod scalar_narrow_f64;
pub mod raw_scalar_narrow;
mod scalar_widen_f32;
pub mod raw_scalar_widen;
mod scalar_f64_operand;
mod scalar_multiply_f64;
pub mod raw_scalar_multiply;
mod scalar_word_arithmetic;
mod scalar_round_f64;
mod scalar_add_f64;
pub mod raw_scalar_add;
pub mod raw_scalar_subtract;
mod scalar_divide_f64;
pub mod raw_scalar_divide;
pub mod floor_f32;
pub mod ceil_f32;
pub mod crt_isa_initializer;
pub mod crt_thread_storage;
pub mod crt_storage_callback;
pub mod crt_storage_owned_free;
pub mod crt_storage_native_return;
pub mod crt_lock_initialization;
pub mod crt_resource_join;
pub mod crt_query_context;
pub mod crt_query_refresh;
pub mod crt_query_resolve;
pub mod crt_query_replace;
pub mod crt_query_calloc;
pub mod crt_query_calloc_join;
pub mod crt_query_calloc_handler;
pub mod crt_query_calloc_handler_frame;
pub mod crt_query_calloc_allocation;
pub mod crt_query_calloc_accessor;
pub mod crt_query_mode;
pub mod crt_query_initialization;
pub mod crt_query_initialization_body;
pub mod crt_query_initialization_join;
pub mod crt_query_null_release;
pub mod crt_query_null_release_join;
pub mod crt_query_owned;
pub mod crt_query_return;
pub mod crt_query_join;
pub mod crt_native_thunk;
pub mod crt_owned_prefix;
pub mod crt_cached_resolver;
pub mod crt_cached_native_return;
pub mod crt_module_publish;
pub mod crt_module_publish_owned;
pub mod crt_module_load;
pub mod crt_module_compare;
pub mod exp_range_continuation;
pub mod matherr_continuation;
pub mod matherr_pointer_initializer;
pub mod norm_f32;
pub mod norm_overflow_continuation;
pub mod norm_nonfinite;
pub mod scalar_cpu_initializer;
pub mod scalar_select_f32;
pub mod logical_f32_outcome;
pub mod range_scalar;
pub mod range_rows;
pub mod selected_atan2_f32;
pub mod selected_coefficient_trig;
pub mod selected_exp;
pub mod selected_exp_f64;
pub mod selected_exp_word_zero;
mod serial_exp;
pub mod ui_exp_serial;
pub mod coefficient_ordinary;
pub mod coefficient_scaled_exp;
pub mod selected_finite_trig_word_zero;
pub mod selected_finite_phase_word_zero;
pub mod selected_log;
pub mod selected_phase;
pub mod selected_phase_word_zero;
pub mod selected_ring_trig;
pub mod square_root;
pub mod square_root_f64;
pub mod raw_scalar_square_root;
pub mod raw_scalar_square_root_f32;
pub mod trig_special_f32;
pub mod trig_special_f64;
pub mod vorbis_power;
pub mod vorbis_arctangent;
pub mod vorbis_codec_math;
pub use self::vorbis_power::{SourceVorbisScalarMath, VorbisScalarError};
pub use self::pcm_probe_math::{SourcePcmProbeMath, PcmProbeMathError,
    PcmProbeArithmeticOperation, PcmProbeTruncationOutcome, PcmProbeWideningOutcome, PcmProbeArithmeticReceipt};
pub use self::f64_value_math::{SourceF64ValueMath, SourceF64ValueError,
    SourceF64ValueOperation, SourceF64ValueOperands, SourceF64ValueCause,
    SourceF64ValueFailure, SourceF64ValueCompletion, SourceF64ValueCandidateReceipt,
    SourceF64ValueResult, SourceF64ValueParts};
pub use self::scalar_truncate::{truncate_f32_to_i32_with_explicit_state,
    truncate_f32_to_i64_with_explicit_state, truncate_f64_to_i32_with_explicit_state,
    SourceF32I32Truncation, SourceF32I64Truncation, SourceF64I32Truncation,
    SourceScalarTruncationError};
pub use self::raw_scalar_truncate::{truncate_f32_to_i32_with_raw_mxcsr,
    truncate_f32_to_i64_with_raw_mxcsr, truncate_f64_to_i64_with_raw_mxcsr,
    truncate_f64_to_i32_with_raw_mxcsr, SourceCvttMxcsrCapabilities,
    SourceRawCvttOutcome, SourceRawCvttError};
pub use self::raw_integer_to_f64::{signed_i64_to_f64_with_raw_mxcsr,
    SourceRawCvtsi2sdOutcome, SourceRawCvtsi2sdError};
pub use self::raw_integer_to_f32::{signed_i64_to_f32_with_raw_mxcsr,
    SourceRawCvtsi2ssError};
pub use self::raw_ucomiss::{ucomiss_f32_with_raw_mxcsr, SourceUcomissFlags,
    SourceRawUcomissOutcome, SourceRawUcomissError};
pub use self::raw_scalar_maximum_f32::{maxss_with_raw_mxcsr,
    SourceRawMaxssOutcome, SourceRawMaxssError};
pub use self::raw_scalar_minimum_f32::{minss_with_raw_mxcsr,
    SourceRawMinssOutcome, SourceRawMinssError};
pub use self::raw_ucomisd::{ucomisd_f64_with_raw_mxcsr, SourceUcomisdFlags,
    SourceRawUcomisdOutcome, SourceRawUcomisdError, ucomisd_f64_with_explicit_state,
    SourceStrictUcomisdOutcome, SourceStrictUcomisdError};
pub use self::raw_scalar_select_f64::{minsd_with_raw_mxcsr, cmpltsd_with_raw_mxcsr,
    SourceF64SelectionOpcode, SourceRawF64SelectionOutcome, SourceRawF64SelectionError};
pub use self::packed_compare_f64::{cmpltpd_with_raw_mxcsr,
    SourceRawCmpltpdOutcome, SourceRawCmpltpdError};
pub use self::raw_scalar_narrow::{cvtsd2ss_with_raw_mxcsr,
    SourceRawCvtsd2ssOutcome, SourceRawCvtsd2ssError};
pub use self::raw_scalar_widen::{cvtss2sd_with_raw_mxcsr,
    SourceRawCvtss2sdOutcome, SourceRawCvtss2sdError};
pub use self::raw_scalar_multiply::{mulsd_with_raw_mxcsr,
    SourceRawMulsdOutcome, SourceRawMulsdError};
pub use self::raw_scalar_add::{addsd_with_raw_mxcsr,
    SourceRawAddsdOutcome, SourceRawAddsdError};
pub use self::raw_scalar_subtract::{subsd_with_raw_mxcsr,
    SourceRawSubsdOutcome, SourceRawSubsdError};
pub use self::raw_scalar_divide::{divsd_with_raw_mxcsr,
    SourceRawDivsdOutcome, SourceRawDivsdError};
pub use self::floor_f32::{SourceFloor580388, SourceFloorF32Error,
    SourceFloorArithmeticAttempt, SourceFloorCallReceipt};
pub use self::ceil_f32::{SourceCeil57dd28, SourceCeilF32Error,
    SourceCeilArithmeticAttempt, SourceCeilCallReceipt};
pub use self::vorbis_arctangent::SourceVorbisArctangent;
pub use self::vorbis_codec_math::{SourceVorbisCodecMath, SourceVorbisCodecWithCallback, VorbisCodecMathError};
pub use self::selected_exp_f64::{SourceF64Exp, SourceF64ExpError, SourceF64ExpCallReceipt};
pub use self::selected_exp_word_zero::{SourceScalarExpWordZero, SourceWordZeroExpError,
    SourceWordZeroExpCallKind, SourceWordZeroExpCallReceipt, SOURCE_RADIAL_EXP_UPPER_F64};
pub use self::ui_exp_serial::{SourceUiSerialExp5b0580, SourceUiSerialExpRequired,
    SourceUiSerialExpError, SourceUiSerialExpCompletion, SourceUiSerialExpParts};
pub use self::coefficient_ordinary::{run_coefficient_ordinary, SourceCoefficientBinaryOpcode,
    SourceCoefficientComparisonFlags, SourceCoefficientExpOperands,
    SourceCoefficientOrdinaryArguments, SourceCoefficientOrdinaryDomainError,
    SourceCoefficientOrdinaryDomainRequired, SourceCoefficientOrdinaryFailure,
    SourceCoefficientOrdinaryRequest, SourceCoefficientOrdinaryServices};
pub use self::coefficient_scaled_exp::{SourceCoefficientScaledExp562834,
    SourceCoefficientScaledExpCause, SourceCoefficientScaledExpError,
    SourceCoefficientScaledExpCompletion, SourceCoefficientPrimitiveError,
    SourceCoefficientReportedOutcome};
pub use self::selected_atan2_f32::{SourceAtan2F32, SourceAtan2F32Error, SourceAtan2F32CallReceipt};
pub use self::norm_f32::{SourceNorm579824, SourceNormF32Error, SourceNormOverflowRequest,
    SourceNormStage, SourceNormStepOperands, SourceNormStepResult, SourceNormArithmeticAttempt,
    SourceNormWideningReceipt, SourceNormCallReceipt};
pub use self::norm_overflow_continuation::{SourceNormOverflow579900, SourceNormOverflowCompletion,
    SourceNormOverflowProgress, SourceNormOverflowError, SourceNormOverflowServiceFailure,
    SourceNormOverflowRejectedFailure};
pub use self::norm_nonfinite::{SourceNormNonfinite57990d, SourceNormNonfiniteError};
pub use self::exp_range_continuation::{SourceExp597c90, SourceExpRangeError,
    SourceExp597c90Progress, SourceExp597c90Completion, SourceExpRangeRequest,
    SourceExpRangeActualReceipt, SourceExp596084Request, SourceExp596084Receipt,
    SourceExp596150Arguments, SourceExpOuterReturnReceipt,
    SourceExp597c90ServiceFailure, SourceExp597c90RejectedFailure};
pub use self::exp_range_continuation::{SourceF32MatherrOuterArguments, SourceF32MatherrOuterReturnReceipt};
pub use self::square_root_f64::{SourceRawSquareRootF64, SourceSqrtF64Error, SourceSqrtF64Completion};
pub use self::raw_scalar_square_root::{sqrtsd_with_raw_mxcsr,
    SourceRawSqrtsdOutcome, SourceRawSqrtsdError};
pub use self::raw_scalar_square_root_f32::{sqrtss_with_raw_mxcsr,
    SourceRawSqrtssOutcome, SourceRawSqrtssError};
pub use self::matherr_pointer_initializer::{SourceCrtPointerInitializer567d24,
    SourceCrtPointerSlot, SourceCrtPointerStore, SourcePointerInitializerRequest,
    SourcePointerInitializerProgress, SourcePointerInitializerCompletion,
    SourcePointerInitializerError, SourcePointerInitializerServiceFailure,
    SourcePointerInitializerRejectedFailure};
pub use self::matherr_pointer_initializer::{SourceCrtTableInitializer567e0c,
    SourceCrtTableReadRole, SourceCrtTableRead, SourceCrtTableCallRole,
    SourceCrtTableCall, SourceCrtTableRequest, SourceCrtTableReceipt,
    SourceCrtTableCompletion, SourceCrtTableProgress, SourceCrtTableError,
    SourceCrtTableServiceFailure, SourceCrtTableRejectedFailure};
pub use self::matherr_pointer_initializer::{SourceCrtStartup55de6c, SourceCrtStartupCall,
    SourceCrtStartupRequest, SourceCrtStartupReceipt, SourceCrtStartupPrefix,
    SourceCrtStartupCompletion, SourceCrtStartupProgress, SourceCrtStartupError,
    SourceCrtStartupServiceFailure, SourceCrtStartupRejectedFailure};
pub use self::selected_ring_trig::{SourceRingTrigWordZero, SourceFloor0CosineWordZero, SourceProjectionTrigWordZero, SourceRingTrigError};
pub use self::selected_ring_trig::{SourceCoefficientTrigWordZeroShort, SourceCoefficientTrigWordZeroShortError};
pub use self::selected_ring_trig::{SourceLargeCosineWordZero, SourceLargeCosineWordZeroError};
pub use self::selected_ring_trig::{SourceFiniteCosineWordZero, SourceFiniteCosineWordZeroError};
pub use self::selected_finite_trig_word_zero::{SourceFiniteTrigWordZero, SourceFiniteTrigWordZeroError,
    SourceFiniteTrigWordZeroOperation, SourceFiniteTrigWordZeroCallReceipt};
pub use self::selected_finite_phase_word_zero::{SourceFinitePhaseWordZero, SourceFinitePhaseWordZeroError,
    SourceFinitePhaseWordZeroStatusFailure};
pub use self::trig_special_f32::{SourceTrigSpecialF32, SourceTrigSpecialFunctionF32,
    SourceTrigSpecialActualReceipt, SourceTrigSpecialCompletion, SourceTrigSpecialProgress,
    SourceTrigSpecialError, SourceTrigSpecialServiceFailure, SourceTrigSpecialRejectedFailure};
pub use self::trig_special_f64::{SourceTrigSpecialF64, SourceTrigSpecialFunctionF64,
    SourceTrigF64OuterArguments, SourceTrigSpecialF64ActualReceipt, SourceTrigSpecialF64Completion,
    SourceTrigSpecialF64Progress, SourceTrigSpecialF64Error, SourceTrigSpecialF64ServiceFailure,
    SourceTrigSpecialF64RejectedFailure};
pub use self::selected_phase_word_zero::{SourcePhaseWordZeroMdct, SourcePhaseWordZeroShort, SourceMdctPhaseWordZeroError};
pub use self::selected_phase_word_zero::{SourcePhaseWordZeroModerate, SourceModeratePhaseWordZeroError};
pub use self::selected_phase_word_zero::{SourcePhaseWordZeroLarge, SourceLargePhaseWordZeroError};
pub use self::scalar_cpu_initializer::{SourceScalarCpuInitializer, SourceScalarCpuInitializerError,
    SourceScalarCpuInitialization, SourceScalarCpuProgress, SourceScalarCpuRequest,
    SourceScalarInitializerWord, SourceScalarInitializerWrite, SuppliedCpuidRecord,
    SuppliedXgetbvRecord, SuppliedScalarCpuRecord};
pub use self::crt_isa_initializer::{SourceCrtGeneralIsa561dd4, SourceCrtIsaRequest,
    SourceCrtIsaRecord, SourceCrtIsaWrite, SourceCrtIsaCompletion, SourceCrtIsaProgress,
    SourceCrtIsaError};
pub use self::crt_thread_storage::{SourceCrtThreadStorage58a4d4, SourceCrtThreadStorageBindings,
    SourceCrtThreadStorageCall, SourceCrtThreadStorageRequest, SourceCrtThreadStorageReceipt,
    SourceCrtThreadStorageEntry, SourceCrtThreadStoragePrefix, SourceCrtThreadStorageCompletion,
    SourceCrtThreadStorageProgress, SourceCrtThreadStorageError, SourceCrtThreadStorageServiceFailure,
    SourceCrtThreadStorageRejectedFailure, SourceCrtStorageThunkBinding,
    SourceCrtStorageThunkBeginError, SourceCrtStorageThunkReturned,
    SourceCrtStorageThunkCompletionError, SourceCrtStorageThunkAccepted};
pub use self::crt_thread_storage::{plan_source_storage_callback_58a6bc,
    SourceCrtStorageCallbackPlan, SourceCrtStorageCallbackReturn};
pub use self::crt_storage_callback::{SourceCrtStorageCallback58a6bc, SourceCrtStorageCallbackEntry,
    SourceCrtStorageFree580c24Request, SourceCrtStorageFree580c24Returned,
    SourceCrtStorageCallbackCompletion, SourceCrtStorageCallbackProgress, SourceCrtStorageCallbackError,
    SourceCrtStorageCallbackServiceFailure, SourceCrtStorageCallbackRejectedFailure};
pub use self::crt_storage_owned_free::{SourceCrtStorageCallbackOwnedFree,
    SourceCrtStorageActualAllocation, SourceCrtStorageActualFreeEntry,
    SourceCrtStorageBorrowedDeleteStep, SourceCrtStorageActualDeleteCompletion,
    SourceCrtStorageAllocationCallbacks, SourceCrtStorageDeleteCallbacks,
    SourceCrtStorageOwnedFreePhase, SourceCrtStorageOwnedFreeBindReason,
    SourceCrtStorageOwnedFreeBindFailure, SourceCrtStorageOwnedFreeError,
    SourceCrtStorageOwnedFreeFailure, SourceCrtStorageOwnedFreeReceiptError,
    SourceCrtStorageOwnedFreeCompletion, SourceCrtStorageOwnedFreeParts};
pub use self::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead,
    SourceCrtNativeRetired, SourceCrtNativeRetObservation, SourceCrtNativeRetExpectation,
    SourceCrtNativeRetMismatch, validate_source_crt_native_ret, SourceCrtNativeReturnFailure,
    SourceCrtNativeRetPhase, SourceCrtNativeRetError, accept_source_crt_native_ret_receipt,
    SourceCrtStorageNativeRetBindings, SourceCrtStorageNativeRetBindReason,
    SourceCrtStorageNativeRetBindFailure, SourceCrtStorageNativeRetPhase, SourceCrtStorageNativeRetError,
    SourceCrtStorageCallbackReturned, SourceCrtStorageNativeRetParts, SourceCrtStorageCallbackNativeRet};
pub use self::crt_cached_native_return::{SourceCrtCachedResolverReturnFrame,
    SourceCrtCachedNativeRetBindings, SourceCrtCachedNativeRetBindReason, SourceCrtCachedNativeRetBindFailure,
    SourceCrtCachedResolverReturned, SourceCrtCachedNativeRetParts, SourceCrtCachedResolverNativeRet};
pub use self::crt_lock_initialization::{SourceCrtLock58d8a0, SourceCrtLockBindings,
    SourceCrtLockRequest, SourceCrtLockReceipt, SourceCrtLockEntry, SourceCrtLockPrefix,
    SourceCrtLockCompletion, SourceCrtLockProgress, SourceCrtLockError,
    SourceCrtLockServiceFailure, SourceCrtLockRejectedFailure, SourceCrtLockThunkBinding,
    SourceCrtLockThunkBeginError, SourceCrtLockThunkCompletionError, SourceCrtLockThunkAccepted};
pub use self::crt_resource_join::{SourceCrtResources564ccc, SourceCrtResourcesEntry,
    SourceCrtResourcesRequest, SourceCrtResourcesReceipt, SourceCrtResourcesPrefix,
    SourceCrtResourcesCompletion, SourceCrtResourcesProgress, SourceCrtResourcesError,
    SourceCrtResourcesServiceFailure, SourceCrtResourcesRejectedFailure};
pub use self::crt_native_thunk::{plan_source_crt_resolver_cache_word, SourceCrtResolverCachePlan,
    SourceCrtNativeThunk, SourceCrtNativeThunkEntry, SourceCrtResolver59bd70Request,
    SourceCrtArgumentOverrides, SourceCrtDispatchStyle, SourceCrtGuard77e920Request,
    SourceCrtThunkFallbackRequest, SourceCrtNativeThunkRequest, SourceCrtNativeThunkReceipt,
    SourceCrtNativeThunkCompletion, SourceCrtNativeThunkReturnedToCaller,
    SourceCrtNativeThunkProgress, SourceCrtNativeThunkError,
    SourceCrtNativeThunkServiceFailure, SourceCrtNativeThunkRejectedFailure,
    SourceCrtThunkResolverBeginError, SourceCrtThunkResolverReturned,
    SourceCrtThunkResolverCompletionError, SourceCrtThunkResolverAccepted,
    SourceCrtThunkOwnedResolverError, SourceCrtThunkOwnedResolverAccepted,
    SourceCrtThunkActualCallerReturn, SourceCrtThunkOwnedReturnError, SourceCrtThunkOwnedReturned,
    SourceCrtOwnedThunkAdmissionError, SourceCrtOwnedThunkAdmission};
pub use self::crt_owned_prefix::{SourceCrtOwnedPrefix, SourceCrtOwnedMemoryObservation,
    SourceCrtOwnedMemoryStep, SourceCrtOwnedMemoryError,
    SourceCrtOwnedControllerCompletion, SourceCrtOwnedControllerCompletionError,
    SourceCrtOwnedPendingChild, SourceCrtOwnedChildBeginError};
pub use self::crt_cached_resolver::{SourceCrtCachedResolver59bd70, SourceCrtCachedResolverRequest,
    SourceCrtCachedResolverReceipt, SourceCrtCachedResolverCompletion, SourceCrtCachedResolverProgress,
    SourceCrtCachedResolverError, SourceCrtCachedResolverServiceFailure, SourceCrtCachedResolverRejectedFailure};
pub use self::crt_cached_resolver::{SourceCrtCachedResolverLoadEntryError,
    SourceCrtCachedResolverLoadCompletionError, SourceCrtCachedResolverLoadAccepted,
    SourceCrtCachedResolverOwnedLoadError, SourceCrtCachedResolverOwnedLoadAccepted};
pub use self::crt_module_publish::{SourceCrtModulePublish59be59, SourceCrtModulePublishRequest,
    SourceCrtModulePublishReceipt, SourceCrtModulePublishCompletion, SourceCrtModulePublishProgress,
    SourceCrtModulePublishError, SourceCrtModulePublishServiceFailure, SourceCrtModulePublishRejectedFailure};
pub use self::crt_module_publish_owned::{SourceCrtOwnedModulePublication, SourceCrtModulePublicationCallbacks,
    SourceCrtModulePublicationActual, SourceCrtModulePublicationPhase, SourceCrtModulePublicationBindFailure,
    SourceCrtModulePublicationError, SourceCrtModulePublicationFailure, SourceCrtModulePublicationResumeError,
    SourceCrtModulePublicationCompletion, SourceCrtModulePublicationParts};
pub use self::crt_module_load::{SourceCrtModuleLoad59bdfb, SourceCrtModuleLoadSaved,
    SourceCrtModuleLoadEntry, SourceCrtModuleLoadNativeKind, SourceCrtModuleLoadNativeCall,
    SourceCrtModuleLoadNativeReceipt, SourceCrtModuleLoadRequest, SourceCrtModuleLoadReceipt,
    SourceCrtModuleLoadHistory, SourceCrtModuleLoadOutcome, SourceCrtModuleLoadCompletion,
    SourceCrtModuleLoadProgress, SourceCrtModuleLoadBeginReason, SourceCrtModuleLoadBeginError,
    SourceCrtModuleLoadError, SourceCrtModuleLoadServiceFailure, SourceCrtModuleLoadRejectedFailure,
    SourceCrtModuleLoadCompareBeginError, SourceCrtModuleCompareReturned,
    SourceCrtModuleLoadCompareCompletionError, SourceCrtModuleLoadCompareAccepted,
    SourceCrtModuleLoadPublicationBinding, SourceCrtModuleLoadPublicationBeginError,
    SourceCrtModuleLoadPublicationCompletionError, SourceCrtModuleLoadPublicationAccepted};
pub use self::crt_module_compare::{SourceCrtModuleCompare593e68, SourceCrtModuleComparePhase,
    SourceCrtModuleCompareRead, SourceCrtModuleCompareReadReceipt, SourceCrtModuleCompareLive,
    SourceCrtModuleCompareCompletion, SourceCrtModuleCompareProgress, SourceCrtModuleCompareError,
    SourceCrtModuleCompareServiceFailure, SourceCrtModuleCompareRejectedFailure};

pub use self::shared_scalar_f32::ScalarF32Architecture;
pub use self::shared_scalar_f32::comparison::fdtest_f32_low_word;
pub use self::scalar_select_f32::{minmax_f32_with_explicit_state, cmpss_with_explicit_state,
    SourceF32MinMaxOpcode, SourceF32Selection, SourceCmpssPredicate,
    SourceF32MaskComparison, SourceF32SelectionError};
pub use self::finite_power::{compare_f32_with_explicit_state, SourceF32CompareOpcode, SourceF32Comparison};
pub use self::finite_power::{plan_source_scale_profile_neutral, source_exponent_bits_profile_neutral};
pub use self::finite_power::{plan_source_notification_profile_neutral, SourceExceptionNotificationPlan};
pub use self::finite_power::{cvtps2pd_with_explicit_state, cvtsd2ss_with_explicit_state,
    cvtsd2ss_with_explicit_status, SourceScalarF32ConversionWithStatus,
    SourcePackedF32ToF64WithState, SourceScalarConversionError};
pub use self::selected_log::{finite_f32_divide_with_explicit_state, normal_f32_divide_with_explicit_state, raw_divss_with_explicit_state};
pub use self::selected_log::{exact_log_exponent_conversion, SourceLogArithmeticStep,
    SourceLogArithmeticFailure, SourceLogFailureCapture, SourceLogObservedInstruction, SourceLogOperationObserver};
pub use self::logical_f32_outcome::{project_scalar_f32_outcome, project_xmm_f32_outcome,
    project_comparison_f32_outcome, SourceScalarF32Projection, SourceXmmF32Projection,
    SourceComparisonF32Projection};

pub use self::finite_power::{CoefficientScaleResult, CoefficientScaledExpBoundary, CoefficientSourceExpBoundary, CoefficientSourceExpEntry, ExplicitExceptionState, FinitePowerServices, MaskedMathError, MathErrorCall, MathErrorCallbackCompletion, MathErrorCallbackFailure, MathErrorCallbackFloat32, MathErrorCallbackPolicy, MathErrorCallbackResponse, MathErrorConversionRequired, MathErrorOperation, MathErrorServiceError, MathErrorWithState, NormalArithmeticError, PowerError, RequiredException, ScalePlan, ScaleWithState, SourceMathErrorCallback};
pub use self::selected_exp::{ExpError, SourceExpEntry, SourceExpEvaluationError, SourceExpPackedReduction, SourceExpPlan, SourceExpReduction, SourceExpRemainder, SourceExpWithState, SourceScalarExpNonzero};
pub use self::selected_exp::{selected_exp_with_explicit_upper_lane,
    selected_exp_with_explicit_upper_lane_receipt, SourceExpDispatchReceipt,
    SourceExpDispatchOutcome, SourceExpDispatchError};
pub use self::selected_coefficient_trig::{ScalarTrigError, ScalarTrigEvaluationError, ScalarTrigReductionWithState, ScalarTrigWithState, SourceCoefficientTrigNonzero};
pub use self::selected_log::{Environment, Error, LogArithmeticError, LogBranch, LogEvaluationError, LogExceptionEvaluationError, SelectedProfilePositiveLog, SourceLogEntry, SourceLogExceptionalOutcome, SourceLogPlan, SourceLogWithState};
pub use self::selected_phase::{PhaseError, PhaseErrorWithState, PhaseWithStatus, PhaseStatusFailure,
    PhaseEvaluationError, PhaseReduction, PhaseReductionWithState, SourcePhaseNonzero};
pub use self::square_root::{SourceSquareRoot, SquareRootError, SquareRootEvaluationError, SquareRootPlan, SquareRootWithState};
