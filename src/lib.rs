pub mod bank;
pub mod bank_reducer;
pub mod canvas;
pub mod codec;
#[cfg(windows)]
pub mod wasapi;
#[cfg(windows)]
pub mod wasapi_devices;
#[cfg(windows)]
pub mod wasapi_render;
#[cfg(windows)]
pub mod wasapi_playback;
#[cfg(windows)]
pub mod wasapi_capture;
#[cfg(windows)]
pub mod wasapi_recording;
#[cfg(windows)]
pub mod wasapi_recording_wave;
#[cfg(windows)]
pub mod wasapi_duplex;
#[cfg(windows)]
pub(crate) mod audio_scheduling;
#[cfg(windows)]
pub mod live_audio;
#[cfg(windows)]
pub mod morph_processor;
#[cfg(windows)]
pub mod wasapi_frame_input;
#[cfg(feature = "desktop")]
pub mod desktop;
pub mod fft;
pub mod generator;
pub mod model;
pub mod userdata;
pub mod secondary_userdata_binding;
pub mod ncwn_parameters;
pub mod ncwn_value_binding;
mod model_tags;
pub mod noise;
pub mod gaussian_noise;
pub mod half_storage;
pub mod matrix_f32;
pub mod matrix_half;
pub mod matrix_policy;
mod matrix_batch;
pub mod vector_f32;
pub mod value_row_allocation;
#[path = "runtime/dsp/row_prepare_values.rs"]
pub mod row_prepare_values;
pub mod normalization;
pub mod pitch_score;
pub mod preset;
pub mod reduction;
pub mod timbre;
pub mod quantized_matrix;

pub type Result<T> = std::result::Result<T, String>;

#[allow(dead_code)]
#[path = "runtime/scalar/src/lib.rs"]
pub mod source_scalar;
pub mod audio_spectrum;
pub mod frontend_gru;
pub mod gru_parameters;
#[path = "runtime/dsp/modl3_values.rs"]
mod modl3_values;
pub mod quantized_gru;
pub mod dct;
pub mod dct_parameters;
pub mod dct_value_binding;
#[path = "runtime/dsp/condition_values.rs"]
mod condition_values;
#[path = "runtime/dsp/model2_operator_binding.rs"]
mod model2_operator_binding;
pub mod condition;
pub mod condition_parameters;
pub mod condition_value_binding;
pub mod condition_userdata_handoff;
pub mod condition_phase_words;
#[path = "runtime/dsp/phase_reallocation.rs"]
pub mod phase_reallocation;
pub mod input_transform_4a640;
pub mod input_transform_value_services;
pub mod input_transform_math_services;
pub mod condition_ncwn_handoff;
pub mod root_conditional_handoff;
pub use paired_adopt::pipeline::root_cepstral_handoff;
pub mod condition_dct;
pub mod condition_gru;
pub mod condition_phase;

#[path = "runtime/dsp/condition_history.rs"]
pub mod condition_history;
#[path = "runtime/dsp/root_binding.rs"]
pub mod root_binding;
#[path = "runtime/dsp/conditional_gate.rs"]
pub mod conditional_gate;
#[path = "runtime/dsp/logical_read_rows.rs"]
pub mod logical_read_rows;
#[path = "runtime/dsp/modm0_sequence_values.rs"]
pub mod modm0_sequence_values;
#[path = "runtime/dsp/modm0_logical_sequence.rs"]
pub mod modm0_logical_sequence;
#[path = "runtime/dsp/moda7_values.rs"]
pub mod moda7_values;
#[path = "runtime/dsp/moda7_projection.rs"]
pub mod moda7_projection;
pub mod moda7_value_services;
#[path = "runtime/dsp/modl0_values.rs"]
pub mod modl0_values;
#[path = "runtime/dsp/modl0_projection.rs"]
pub mod modl0_projection;
pub mod modl0_value_services;
pub mod modl0_matrix_value;
pub mod modl0_float_parameters;
pub mod modl0_quantized_parameters;
mod value_call_owners;
#[path = "runtime/dsp/model2_logical_chain.rs"]
pub mod model2_logical_chain;
pub mod model2_value_reset;
pub mod model2_value_binding;
pub mod model2_float_parameters;
pub mod model2_quantized_parameters;
#[path = "runtime/dsp/row_arithmetic.rs"]
pub mod row_arithmetic;
#[path = "runtime/dsp/activation.rs"]
pub mod activation;
#[path = "runtime/dsp/backend_factory.rs"]
pub mod backend_factory;
#[path = "runtime/dsp/conditional_ncwn.rs"]
pub mod conditional_ncwn;
pub mod conditioned_stages;
pub mod conditioned_model;
#[path = "runtime/dsp/cepstral_values.rs"]
mod cepstral_values;
pub mod cepstral;
pub mod cepstral_parameters;
pub mod cepstral_value_binding;
pub mod cepstral_runtime_parameters;
pub mod cepstral_fanout_initialization;
pub mod cepstral_userdata_binding;
pub mod sequence_parameters;
pub mod sequence_value_binding;
pub mod sequence_userdata_binding;
pub mod sequence_value_reset;
pub mod cepstral_model;
pub mod cepstral_queue_model;
#[path = "runtime/dsp/projected_conv_binding.rs"]
pub mod projected_conv_binding;
#[path = "runtime/dsp/streaming_projection.rs"]
pub mod streaming_projection;
#[path = "runtime/dsp/matrix_value_traits.rs"]
pub mod matrix_value_traits;
#[path = "runtime/dsp/row_pool_values.rs"]
pub mod row_pool_values;
#[path = "runtime/dsp/row_acquire_values.rs"]
mod row_acquire_values;
#[path = "runtime/dsp/row_put_values.rs"]
mod row_put_values;
#[path = "runtime/dsp/condition_deque_clear_values.rs"]
mod condition_deque_clear_values;
pub mod streaming_history_clear;
#[path = "runtime/dsp/streaming_value_services.rs"]
pub mod streaming_value_services;
pub mod streaming_rows;
pub mod streaming_module;
pub mod streaming_matrix;
pub mod streaming_quantized_matrix;
pub mod streaming_parameters;
pub mod streaming_bank_parameters;
pub mod streaming_value_binding;
pub mod spectral_encoder;
pub mod feature_projection;
pub mod temporal_mixer;
pub mod frontend_producer;
pub mod frontend_gain;
pub mod frontend_energy_window_math;
pub mod frontend_gated_energy_math;
pub mod frontend_gain_count_math;
pub mod frontend_gain_interpolation_math;
pub mod frontend_gain_log_suffix_math;
pub mod frontend_gain_statistics;
pub mod frontend_gain_log_rows;
pub mod voice_library;
pub mod sparse_linear;
pub mod pitch_temporal;
pub mod conditional_head;
mod tone_noise;
pub mod tone;
pub mod pitch_spectral_encoder;
pub mod pitch_audio_spectrum;
pub mod pitch_selected_head;
pub(crate) mod pitch_display;

pub mod generation_curve;
pub mod source_pitch_join;
pub mod analysis_frontend;
pub mod generation_schedule;
pub mod trajectory_pitch;
pub mod voice_generation;
pub mod wav48k;
pub mod wave_output;
pub mod ordinary_audio;
mod resample44100;

#[path = "runtime/dsp/ltv_block.rs"]
mod ltv_block;
#[path = "runtime/dsp/ltv_history.rs"]
mod ltv_history;
pub mod ltv_render;
#[path = "runtime/dsp/ltv_coefficient_values.rs"]
mod ltv_coefficient_values;
pub mod ltv_synthesis;
pub mod ltv_coefficient_math;
#[path = "runtime/dsp/ltv_filter_values.rs"]
mod ltv_filter_values;
pub mod ltv_filter;
pub mod ltv_value_binding;
pub mod ltv_value_initialization;
#[path = "runtime/dsp/ltv_transform_selection.rs"]
pub mod ltv_first_call;
pub mod ltv_transform_initialization;
#[path = "runtime/dsp/vector_environment.rs"]
pub mod vector_environment;
#[path = "runtime/dsp/ltv_transform_rounding.rs"]
pub mod ltv_transform_rounding;
#[path = "runtime/dsp/ltv_transform_log.rs"]
pub mod ltv_transform_log;
pub mod ltv_transform_math;
pub mod ltv_filter_model;
#[path = "runtime/dsp/ltv_caller_second_values.rs"]
mod ltv_caller_second_values;
pub mod ltv_second_refill;
#[path = "runtime/dsp/paired_input_values.rs"]
mod paired_input_values;
pub mod paired_input;
pub mod paired_value_rows;
#[path = "runtime/dsp/paired_feature_values.rs"]
mod paired_feature_values;
pub mod paired_features;
pub mod paired_feature_row;
#[path = "runtime/dsp/paired_vector_checkout_values.rs"]
mod paired_vector_checkout_values;
#[path = "runtime/dsp/paired_vector_return_values.rs"]
mod paired_vector_return_values;
pub mod paired_frames_pool;
#[path = "runtime/dsp/paired_condition_values.rs"]
mod paired_condition_values;
pub mod paired_condition;
pub mod paired_condition_worker;
mod module_output;
pub mod paired_projection_module;
pub mod paired_post_join;
#[path = "runtime/dsp/paired_adopt_values.rs"]
mod paired_adopt_values;
pub mod paired_adopt;
#[path = "runtime/dsp/paired_constructor_reset_values.rs"]
pub mod paired_constructor_reset_values;
pub mod vocoder_value_binding;
#[path = "runtime/dsp/signal_rate_values.rs"]
pub mod signal_rate;
pub mod vocoder_rate;
pub mod vocoder_userdata_binding;
pub mod vocoder_storage_binding;
#[path = "runtime/dsp/paired_interpolation_values.rs"]
mod paired_interpolation_values;
pub mod paired_interpolation;
#[path = "runtime/dsp/header_append_values.rs"]
mod header_append_values;
#[path = "runtime/dsp/paired_scalar_values.rs"]
mod paired_scalar_values;
pub mod paired_scalar_packing;
#[path = "runtime/dsp/paired_post_values.rs"]
mod paired_post_values;
pub mod paired_post_noise;
pub mod paired_post;
#[path = "runtime/dsp/paired_retirement_values.rs"]
mod paired_retirement_values;
#[path = "runtime/dsp/ltv_paired_values.rs"]
mod ltv_paired_values;
#[path = "runtime/dsp/ltv_emission_values.rs"]
mod ltv_emission_values;
pub mod ltv_stream;
pub mod ltv_output_values;
#[path = "runtime/dsp/ltv_window_values.rs"]
mod ltv_window_values;
pub mod ltv_analysis_window;
pub mod prototype_controls;
pub mod current_condition;
mod prepare_quality;
#[cfg(windows)]
pub mod plugin_bridge;
#[cfg(all(windows, feature = "desktop"))]
pub mod plugin_editor;
#[cfg(feature = "desktop")]
mod editor_session;
pub mod continuous_dsp;
pub mod prototype_synthesis_networks;
