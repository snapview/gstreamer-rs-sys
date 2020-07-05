// This file was generated by gir (https://github.com/gtk-rs/gir @ 60cbef0)
// from gir-files (https://github.com/gtk-rs/gir-files @ 1e16d41)
// DO NOT EDIT

extern crate gstreamer_audio_sys;
extern crate shell_words;
extern crate tempfile;
use gstreamer_audio_sys::*;
use std::env;
use std::error::Error;
use std::mem::{align_of, size_of};
use std::path::Path;
use std::process::Command;
use std::str;
use tempfile::Builder;

static PACKAGES: &[&str] = &["gstreamer-audio-1.0"];

#[derive(Clone, Debug)]
struct Compiler {
    pub args: Vec<String>,
}

impl Compiler {
    pub fn new() -> Result<Compiler, Box<dyn Error>> {
        let mut args = get_var("CC", "cc")?;
        args.push("-Wno-deprecated-declarations".to_owned());
        // For %z support in printf when using MinGW.
        args.push("-D__USE_MINGW_ANSI_STDIO".to_owned());
        args.extend(get_var("CFLAGS", "")?);
        args.extend(get_var("CPPFLAGS", "")?);
        args.extend(pkg_config_cflags(PACKAGES)?);
        Ok(Compiler { args })
    }

    pub fn define<'a, V: Into<Option<&'a str>>>(&mut self, var: &str, val: V) {
        let arg = match val.into() {
            None => format!("-D{}", var),
            Some(val) => format!("-D{}={}", var, val),
        };
        self.args.push(arg);
    }

    pub fn compile(&self, src: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
        let mut cmd = self.to_command();
        cmd.arg(src);
        cmd.arg("-o");
        cmd.arg(out);
        let status = cmd.spawn()?.wait()?;
        if !status.success() {
            return Err(format!("compilation command {:?} failed, {}", &cmd, status).into());
        }
        Ok(())
    }

    fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.args[0]);
        cmd.args(&self.args[1..]);
        cmd
    }
}

fn get_var(name: &str, default: &str) -> Result<Vec<String>, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) => Ok(shell_words::split(&value)?),
        Err(env::VarError::NotPresent) => Ok(shell_words::split(default)?),
        Err(err) => Err(format!("{} {}", name, err).into()),
    }
}

fn pkg_config_cflags(packages: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }
    let mut cmd = Command::new("pkg-config");
    cmd.arg("--cflags");
    cmd.args(packages);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(format!("command {:?} returned {}", &cmd, out.status).into());
    }
    let stdout = str::from_utf8(&out.stdout)?;
    Ok(shell_words::split(stdout.trim())?)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Layout {
    size: usize,
    alignment: usize,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct Results {
    /// Number of successfully completed tests.
    passed: usize,
    /// Total number of failed tests (including those that failed to compile).
    failed: usize,
    /// Number of tests that failed to compile.
    failed_to_compile: usize,
}

impl Results {
    fn record_passed(&mut self) {
        self.passed += 1;
    }
    fn record_failed(&mut self) {
        self.failed += 1;
    }
    fn record_failed_to_compile(&mut self) {
        self.failed += 1;
        self.failed_to_compile += 1;
    }
    fn summary(&self) -> String {
        format!(
            "{} passed; {} failed (compilation errors: {})",
            self.passed, self.failed, self.failed_to_compile
        )
    }
    fn expect_total_success(&self) {
        if self.failed == 0 {
            println!("OK: {}", self.summary());
        } else {
            panic!("FAILED: {}", self.summary());
        };
    }
}

#[test]
fn cross_validate_constants_with_c() {
    let tmpdir = Builder::new()
        .prefix("abi")
        .tempdir()
        .expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!(
        "1",
        get_c_value(tmpdir.path(), &cc, "1").expect("C constant"),
        "failed to obtain correct constant value for 1"
    );

    let mut results: Results = Default::default();
    for (i, &(name, rust_value)) in RUST_CONSTANTS.iter().enumerate() {
        match get_c_value(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            }
            Ok(ref c_value) => {
                if rust_value == c_value {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!(
                        "Constant value mismatch for {}\nRust: {:?}\nC:    {:?}",
                        name, rust_value, c_value
                    );
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("constants ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

#[test]
fn cross_validate_layout_with_c() {
    let tmpdir = Builder::new()
        .prefix("abi")
        .tempdir()
        .expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!(
        Layout {
            size: 1,
            alignment: 1
        },
        get_c_layout(tmpdir.path(), &cc, "char").expect("C layout"),
        "failed to obtain correct layout for char type"
    );

    let mut results: Results = Default::default();
    for (i, &(name, rust_layout)) in RUST_LAYOUTS.iter().enumerate() {
        match get_c_layout(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            }
            Ok(c_layout) => {
                if rust_layout == c_layout {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!(
                        "Layout mismatch for {}\nRust: {:?}\nC:    {:?}",
                        name, rust_layout, &c_layout
                    );
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("layout    ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

fn get_c_layout(dir: &Path, cc: &Compiler, name: &str) -> Result<Layout, Box<dyn Error>> {
    let exe = dir.join("layout");
    let mut cc = cc.clone();
    cc.define("ABI_TYPE_NAME", name);
    cc.compile(Path::new("tests/layout.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}", &abi_cmd, &output).into());
    }

    let stdout = str::from_utf8(&output.stdout)?;
    let mut words = stdout.trim().split_whitespace();
    let size = words.next().unwrap().parse().unwrap();
    let alignment = words.next().unwrap().parse().unwrap();
    Ok(Layout { size, alignment })
}

fn get_c_value(dir: &Path, cc: &Compiler, name: &str) -> Result<String, Box<dyn Error>> {
    let exe = dir.join("constant");
    let mut cc = cc.clone();
    cc.define("ABI_CONSTANT_NAME", name);
    cc.compile(Path::new("tests/constant.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}", &abi_cmd, &output).into());
    }

    let output = str::from_utf8(&output.stdout)?.trim();
    if !output.starts_with("###gir test###") || !output.ends_with("###gir test###") {
        return Err(format!(
            "command {:?} return invalid output, {:?}",
            &abi_cmd, &output
        )
        .into());
    }

    Ok(String::from(&output[14..(output.len() - 14)]))
}

const RUST_LAYOUTS: &[(&str, Layout)] = &[
    (
        "GstAudioAggregator",
        Layout {
            size: size_of::<GstAudioAggregator>(),
            alignment: align_of::<GstAudioAggregator>(),
        },
    ),
    (
        "GstAudioAggregatorClass",
        Layout {
            size: size_of::<GstAudioAggregatorClass>(),
            alignment: align_of::<GstAudioAggregatorClass>(),
        },
    ),
    (
        "GstAudioAggregatorConvertPad",
        Layout {
            size: size_of::<GstAudioAggregatorConvertPad>(),
            alignment: align_of::<GstAudioAggregatorConvertPad>(),
        },
    ),
    (
        "GstAudioAggregatorConvertPadClass",
        Layout {
            size: size_of::<GstAudioAggregatorConvertPadClass>(),
            alignment: align_of::<GstAudioAggregatorConvertPadClass>(),
        },
    ),
    (
        "GstAudioAggregatorPad",
        Layout {
            size: size_of::<GstAudioAggregatorPad>(),
            alignment: align_of::<GstAudioAggregatorPad>(),
        },
    ),
    (
        "GstAudioAggregatorPadClass",
        Layout {
            size: size_of::<GstAudioAggregatorPadClass>(),
            alignment: align_of::<GstAudioAggregatorPadClass>(),
        },
    ),
    (
        "GstAudioBaseSink",
        Layout {
            size: size_of::<GstAudioBaseSink>(),
            alignment: align_of::<GstAudioBaseSink>(),
        },
    ),
    (
        "GstAudioBaseSinkClass",
        Layout {
            size: size_of::<GstAudioBaseSinkClass>(),
            alignment: align_of::<GstAudioBaseSinkClass>(),
        },
    ),
    (
        "GstAudioBaseSinkDiscontReason",
        Layout {
            size: size_of::<GstAudioBaseSinkDiscontReason>(),
            alignment: align_of::<GstAudioBaseSinkDiscontReason>(),
        },
    ),
    (
        "GstAudioBaseSinkSlaveMethod",
        Layout {
            size: size_of::<GstAudioBaseSinkSlaveMethod>(),
            alignment: align_of::<GstAudioBaseSinkSlaveMethod>(),
        },
    ),
    (
        "GstAudioBaseSrc",
        Layout {
            size: size_of::<GstAudioBaseSrc>(),
            alignment: align_of::<GstAudioBaseSrc>(),
        },
    ),
    (
        "GstAudioBaseSrcClass",
        Layout {
            size: size_of::<GstAudioBaseSrcClass>(),
            alignment: align_of::<GstAudioBaseSrcClass>(),
        },
    ),
    (
        "GstAudioBaseSrcSlaveMethod",
        Layout {
            size: size_of::<GstAudioBaseSrcSlaveMethod>(),
            alignment: align_of::<GstAudioBaseSrcSlaveMethod>(),
        },
    ),
    (
        "GstAudioBuffer",
        Layout {
            size: size_of::<GstAudioBuffer>(),
            alignment: align_of::<GstAudioBuffer>(),
        },
    ),
    (
        "GstAudioCdSrc",
        Layout {
            size: size_of::<GstAudioCdSrc>(),
            alignment: align_of::<GstAudioCdSrc>(),
        },
    ),
    (
        "GstAudioCdSrcClass",
        Layout {
            size: size_of::<GstAudioCdSrcClass>(),
            alignment: align_of::<GstAudioCdSrcClass>(),
        },
    ),
    (
        "GstAudioCdSrcMode",
        Layout {
            size: size_of::<GstAudioCdSrcMode>(),
            alignment: align_of::<GstAudioCdSrcMode>(),
        },
    ),
    (
        "GstAudioCdSrcTrack",
        Layout {
            size: size_of::<GstAudioCdSrcTrack>(),
            alignment: align_of::<GstAudioCdSrcTrack>(),
        },
    ),
    (
        "GstAudioChannelMixerFlags",
        Layout {
            size: size_of::<GstAudioChannelMixerFlags>(),
            alignment: align_of::<GstAudioChannelMixerFlags>(),
        },
    ),
    (
        "GstAudioChannelPosition",
        Layout {
            size: size_of::<GstAudioChannelPosition>(),
            alignment: align_of::<GstAudioChannelPosition>(),
        },
    ),
    (
        "GstAudioClippingMeta",
        Layout {
            size: size_of::<GstAudioClippingMeta>(),
            alignment: align_of::<GstAudioClippingMeta>(),
        },
    ),
    (
        "GstAudioClock",
        Layout {
            size: size_of::<GstAudioClock>(),
            alignment: align_of::<GstAudioClock>(),
        },
    ),
    (
        "GstAudioClockClass",
        Layout {
            size: size_of::<GstAudioClockClass>(),
            alignment: align_of::<GstAudioClockClass>(),
        },
    ),
    (
        "GstAudioConverterFlags",
        Layout {
            size: size_of::<GstAudioConverterFlags>(),
            alignment: align_of::<GstAudioConverterFlags>(),
        },
    ),
    (
        "GstAudioDecoder",
        Layout {
            size: size_of::<GstAudioDecoder>(),
            alignment: align_of::<GstAudioDecoder>(),
        },
    ),
    (
        "GstAudioDecoderClass",
        Layout {
            size: size_of::<GstAudioDecoderClass>(),
            alignment: align_of::<GstAudioDecoderClass>(),
        },
    ),
    (
        "GstAudioDitherMethod",
        Layout {
            size: size_of::<GstAudioDitherMethod>(),
            alignment: align_of::<GstAudioDitherMethod>(),
        },
    ),
    (
        "GstAudioDownmixMeta",
        Layout {
            size: size_of::<GstAudioDownmixMeta>(),
            alignment: align_of::<GstAudioDownmixMeta>(),
        },
    ),
    (
        "GstAudioEncoder",
        Layout {
            size: size_of::<GstAudioEncoder>(),
            alignment: align_of::<GstAudioEncoder>(),
        },
    ),
    (
        "GstAudioEncoderClass",
        Layout {
            size: size_of::<GstAudioEncoderClass>(),
            alignment: align_of::<GstAudioEncoderClass>(),
        },
    ),
    (
        "GstAudioFilter",
        Layout {
            size: size_of::<GstAudioFilter>(),
            alignment: align_of::<GstAudioFilter>(),
        },
    ),
    (
        "GstAudioFilterClass",
        Layout {
            size: size_of::<GstAudioFilterClass>(),
            alignment: align_of::<GstAudioFilterClass>(),
        },
    ),
    (
        "GstAudioFlags",
        Layout {
            size: size_of::<GstAudioFlags>(),
            alignment: align_of::<GstAudioFlags>(),
        },
    ),
    (
        "GstAudioFormat",
        Layout {
            size: size_of::<GstAudioFormat>(),
            alignment: align_of::<GstAudioFormat>(),
        },
    ),
    (
        "GstAudioFormatFlags",
        Layout {
            size: size_of::<GstAudioFormatFlags>(),
            alignment: align_of::<GstAudioFormatFlags>(),
        },
    ),
    (
        "GstAudioFormatInfo",
        Layout {
            size: size_of::<GstAudioFormatInfo>(),
            alignment: align_of::<GstAudioFormatInfo>(),
        },
    ),
    (
        "GstAudioInfo",
        Layout {
            size: size_of::<GstAudioInfo>(),
            alignment: align_of::<GstAudioInfo>(),
        },
    ),
    (
        "GstAudioLayout",
        Layout {
            size: size_of::<GstAudioLayout>(),
            alignment: align_of::<GstAudioLayout>(),
        },
    ),
    (
        "GstAudioMeta",
        Layout {
            size: size_of::<GstAudioMeta>(),
            alignment: align_of::<GstAudioMeta>(),
        },
    ),
    (
        "GstAudioNoiseShapingMethod",
        Layout {
            size: size_of::<GstAudioNoiseShapingMethod>(),
            alignment: align_of::<GstAudioNoiseShapingMethod>(),
        },
    ),
    (
        "GstAudioPackFlags",
        Layout {
            size: size_of::<GstAudioPackFlags>(),
            alignment: align_of::<GstAudioPackFlags>(),
        },
    ),
    (
        "GstAudioQuantizeFlags",
        Layout {
            size: size_of::<GstAudioQuantizeFlags>(),
            alignment: align_of::<GstAudioQuantizeFlags>(),
        },
    ),
    (
        "GstAudioResamplerFilterInterpolation",
        Layout {
            size: size_of::<GstAudioResamplerFilterInterpolation>(),
            alignment: align_of::<GstAudioResamplerFilterInterpolation>(),
        },
    ),
    (
        "GstAudioResamplerFilterMode",
        Layout {
            size: size_of::<GstAudioResamplerFilterMode>(),
            alignment: align_of::<GstAudioResamplerFilterMode>(),
        },
    ),
    (
        "GstAudioResamplerFlags",
        Layout {
            size: size_of::<GstAudioResamplerFlags>(),
            alignment: align_of::<GstAudioResamplerFlags>(),
        },
    ),
    (
        "GstAudioResamplerMethod",
        Layout {
            size: size_of::<GstAudioResamplerMethod>(),
            alignment: align_of::<GstAudioResamplerMethod>(),
        },
    ),
    (
        "GstAudioRingBuffer",
        Layout {
            size: size_of::<GstAudioRingBuffer>(),
            alignment: align_of::<GstAudioRingBuffer>(),
        },
    ),
    (
        "GstAudioRingBufferClass",
        Layout {
            size: size_of::<GstAudioRingBufferClass>(),
            alignment: align_of::<GstAudioRingBufferClass>(),
        },
    ),
    (
        "GstAudioRingBufferFormatType",
        Layout {
            size: size_of::<GstAudioRingBufferFormatType>(),
            alignment: align_of::<GstAudioRingBufferFormatType>(),
        },
    ),
    (
        "GstAudioRingBufferSpec",
        Layout {
            size: size_of::<GstAudioRingBufferSpec>(),
            alignment: align_of::<GstAudioRingBufferSpec>(),
        },
    ),
    (
        "GstAudioRingBufferState",
        Layout {
            size: size_of::<GstAudioRingBufferState>(),
            alignment: align_of::<GstAudioRingBufferState>(),
        },
    ),
    (
        "GstAudioSink",
        Layout {
            size: size_of::<GstAudioSink>(),
            alignment: align_of::<GstAudioSink>(),
        },
    ),
    (
        "GstAudioSinkClass",
        Layout {
            size: size_of::<GstAudioSinkClass>(),
            alignment: align_of::<GstAudioSinkClass>(),
        },
    ),
    (
        "GstAudioSinkClassExtension",
        Layout {
            size: size_of::<GstAudioSinkClassExtension>(),
            alignment: align_of::<GstAudioSinkClassExtension>(),
        },
    ),
    (
        "GstAudioSrc",
        Layout {
            size: size_of::<GstAudioSrc>(),
            alignment: align_of::<GstAudioSrc>(),
        },
    ),
    (
        "GstAudioSrcClass",
        Layout {
            size: size_of::<GstAudioSrcClass>(),
            alignment: align_of::<GstAudioSrcClass>(),
        },
    ),
    (
        "GstStreamVolumeFormat",
        Layout {
            size: size_of::<GstStreamVolumeFormat>(),
            alignment: align_of::<GstStreamVolumeFormat>(),
        },
    ),
    (
        "GstStreamVolumeInterface",
        Layout {
            size: size_of::<GstStreamVolumeInterface>(),
            alignment: align_of::<GstStreamVolumeInterface>(),
        },
    ),
];

const RUST_CONSTANTS: &[(&str, &str)] = &[
    ("(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_ALIGNMENT", "4"),
    (
        "(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_DEVICE_FAILURE",
        "5",
    ),
    ("(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_FLUSH", "2"),
    ("(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_NEW_CAPS", "1"),
    ("(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_NO_DISCONT", "0"),
    (
        "(gint) GST_AUDIO_BASE_SINK_DISCONT_REASON_SYNC_LATENCY",
        "3",
    ),
    ("(gint) GST_AUDIO_BASE_SINK_SLAVE_CUSTOM", "3"),
    ("(gint) GST_AUDIO_BASE_SINK_SLAVE_NONE", "2"),
    ("(gint) GST_AUDIO_BASE_SINK_SLAVE_RESAMPLE", "0"),
    ("(gint) GST_AUDIO_BASE_SINK_SLAVE_SKEW", "1"),
    ("(gint) GST_AUDIO_BASE_SRC_SLAVE_NONE", "3"),
    ("(gint) GST_AUDIO_BASE_SRC_SLAVE_RESAMPLE", "0"),
    ("(gint) GST_AUDIO_BASE_SRC_SLAVE_RE_TIMESTAMP", "1"),
    ("(gint) GST_AUDIO_BASE_SRC_SLAVE_SKEW", "2"),
    ("(gint) GST_AUDIO_CD_SRC_MODE_CONTINUOUS", "1"),
    ("(gint) GST_AUDIO_CD_SRC_MODE_NORMAL", "0"),
    ("GST_AUDIO_CHANNELS_RANGE", "(int) [ 1, max ]"),
    ("(guint) GST_AUDIO_CHANNEL_MIXER_FLAGS_NONE", "0"),
    (
        "(guint) GST_AUDIO_CHANNEL_MIXER_FLAGS_NON_INTERLEAVED_IN",
        "1",
    ),
    (
        "(guint) GST_AUDIO_CHANNEL_MIXER_FLAGS_NON_INTERLEAVED_OUT",
        "2",
    ),
    ("(guint) GST_AUDIO_CHANNEL_MIXER_FLAGS_UNPOSITIONED_IN", "4"),
    (
        "(guint) GST_AUDIO_CHANNEL_MIXER_FLAGS_UNPOSITIONED_OUT",
        "8",
    ),
    (
        "(gint) GST_AUDIO_CHANNEL_POSITION_BOTTOM_FRONT_CENTER",
        "21",
    ),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_BOTTOM_FRONT_LEFT", "22"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_BOTTOM_FRONT_RIGHT", "23"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_FRONT_CENTER", "2"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_FRONT_LEFT", "0"),
    (
        "(gint) GST_AUDIO_CHANNEL_POSITION_FRONT_LEFT_OF_CENTER",
        "6",
    ),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_FRONT_RIGHT", "1"),
    (
        "(gint) GST_AUDIO_CHANNEL_POSITION_FRONT_RIGHT_OF_CENTER",
        "7",
    ),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_INVALID", "-1"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_LFE1", "3"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_LFE2", "9"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_MONO", "-2"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_NONE", "-3"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_REAR_CENTER", "8"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_REAR_LEFT", "4"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_REAR_RIGHT", "5"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_SIDE_LEFT", "10"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_SIDE_RIGHT", "11"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_SURROUND_LEFT", "26"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_SURROUND_RIGHT", "27"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_CENTER", "15"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_FRONT_CENTER", "14"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_FRONT_LEFT", "12"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_FRONT_RIGHT", "13"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_REAR_CENTER", "20"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_REAR_LEFT", "16"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_REAR_RIGHT", "17"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_SIDE_LEFT", "18"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_TOP_SIDE_RIGHT", "19"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_WIDE_LEFT", "24"),
    ("(gint) GST_AUDIO_CHANNEL_POSITION_WIDE_RIGHT", "25"),
    ("(guint) GST_AUDIO_CONVERTER_FLAG_IN_WRITABLE", "1"),
    ("(guint) GST_AUDIO_CONVERTER_FLAG_NONE", "0"),
    ("(guint) GST_AUDIO_CONVERTER_FLAG_VARIABLE_RATE", "2"),
    (
        "GST_AUDIO_CONVERTER_OPT_DITHER_METHOD",
        "GstAudioConverter.dither-method",
    ),
    (
        "GST_AUDIO_CONVERTER_OPT_MIX_MATRIX",
        "GstAudioConverter.mix-matrix",
    ),
    (
        "GST_AUDIO_CONVERTER_OPT_NOISE_SHAPING_METHOD",
        "GstAudioConverter.noise-shaping-method",
    ),
    (
        "GST_AUDIO_CONVERTER_OPT_QUANTIZATION",
        "GstAudioConverter.quantization",
    ),
    (
        "GST_AUDIO_CONVERTER_OPT_RESAMPLER_METHOD",
        "GstAudioConverter.resampler-method",
    ),
    ("GST_AUDIO_DECODER_MAX_ERRORS", "10"),
    ("GST_AUDIO_DECODER_SINK_NAME", "sink"),
    ("GST_AUDIO_DECODER_SRC_NAME", "src"),
    ("GST_AUDIO_DEF_CHANNELS", "2"),
    ("GST_AUDIO_DEF_FORMAT", "S16LE"),
    ("GST_AUDIO_DEF_RATE", "44100"),
    ("(gint) GST_AUDIO_DITHER_NONE", "0"),
    ("(gint) GST_AUDIO_DITHER_RPDF", "1"),
    ("(gint) GST_AUDIO_DITHER_TPDF", "2"),
    ("(gint) GST_AUDIO_DITHER_TPDF_HF", "3"),
    ("GST_AUDIO_ENCODER_SINK_NAME", "sink"),
    ("GST_AUDIO_ENCODER_SRC_NAME", "src"),
    ("(guint) GST_AUDIO_FLAG_NONE", "0"),
    ("(guint) GST_AUDIO_FLAG_UNPOSITIONED", "1"),
    ("(gint) GST_AUDIO_FORMAT_ENCODED", "1"),
    ("(gint) GST_AUDIO_FORMAT_F32", "28"),
    ("(gint) GST_AUDIO_FORMAT_F32BE", "29"),
    ("(gint) GST_AUDIO_FORMAT_F32LE", "28"),
    ("(gint) GST_AUDIO_FORMAT_F64", "30"),
    ("(gint) GST_AUDIO_FORMAT_F64BE", "31"),
    ("(gint) GST_AUDIO_FORMAT_F64LE", "30"),
    ("(guint) GST_AUDIO_FORMAT_FLAG_COMPLEX", "16"),
    ("(guint) GST_AUDIO_FORMAT_FLAG_FLOAT", "2"),
    ("(guint) GST_AUDIO_FORMAT_FLAG_INTEGER", "1"),
    ("(guint) GST_AUDIO_FORMAT_FLAG_SIGNED", "4"),
    ("(guint) GST_AUDIO_FORMAT_FLAG_UNPACK", "32"),
    ("(gint) GST_AUDIO_FORMAT_S16", "4"),
    ("(gint) GST_AUDIO_FORMAT_S16BE", "5"),
    ("(gint) GST_AUDIO_FORMAT_S16LE", "4"),
    ("(gint) GST_AUDIO_FORMAT_S18", "24"),
    ("(gint) GST_AUDIO_FORMAT_S18BE", "25"),
    ("(gint) GST_AUDIO_FORMAT_S18LE", "24"),
    ("(gint) GST_AUDIO_FORMAT_S20", "20"),
    ("(gint) GST_AUDIO_FORMAT_S20BE", "21"),
    ("(gint) GST_AUDIO_FORMAT_S20LE", "20"),
    ("(gint) GST_AUDIO_FORMAT_S24", "16"),
    ("(gint) GST_AUDIO_FORMAT_S24BE", "17"),
    ("(gint) GST_AUDIO_FORMAT_S24LE", "16"),
    ("(gint) GST_AUDIO_FORMAT_S24_32", "8"),
    ("(gint) GST_AUDIO_FORMAT_S24_32BE", "9"),
    ("(gint) GST_AUDIO_FORMAT_S24_32LE", "8"),
    ("(gint) GST_AUDIO_FORMAT_S32", "12"),
    ("(gint) GST_AUDIO_FORMAT_S32BE", "13"),
    ("(gint) GST_AUDIO_FORMAT_S32LE", "12"),
    ("(gint) GST_AUDIO_FORMAT_S8", "2"),
    ("(gint) GST_AUDIO_FORMAT_U16", "6"),
    ("(gint) GST_AUDIO_FORMAT_U16BE", "7"),
    ("(gint) GST_AUDIO_FORMAT_U16LE", "6"),
    ("(gint) GST_AUDIO_FORMAT_U18", "26"),
    ("(gint) GST_AUDIO_FORMAT_U18BE", "27"),
    ("(gint) GST_AUDIO_FORMAT_U18LE", "26"),
    ("(gint) GST_AUDIO_FORMAT_U20", "22"),
    ("(gint) GST_AUDIO_FORMAT_U20BE", "23"),
    ("(gint) GST_AUDIO_FORMAT_U20LE", "22"),
    ("(gint) GST_AUDIO_FORMAT_U24", "18"),
    ("(gint) GST_AUDIO_FORMAT_U24BE", "19"),
    ("(gint) GST_AUDIO_FORMAT_U24LE", "18"),
    ("(gint) GST_AUDIO_FORMAT_U24_32", "10"),
    ("(gint) GST_AUDIO_FORMAT_U24_32BE", "11"),
    ("(gint) GST_AUDIO_FORMAT_U24_32LE", "10"),
    ("(gint) GST_AUDIO_FORMAT_U32", "14"),
    ("(gint) GST_AUDIO_FORMAT_U32BE", "15"),
    ("(gint) GST_AUDIO_FORMAT_U32LE", "14"),
    ("(gint) GST_AUDIO_FORMAT_U8", "3"),
    ("(gint) GST_AUDIO_FORMAT_UNKNOWN", "0"),
    ("(gint) GST_AUDIO_LAYOUT_INTERLEAVED", "0"),
    ("(gint) GST_AUDIO_LAYOUT_NON_INTERLEAVED", "1"),
    ("(gint) GST_AUDIO_NOISE_SHAPING_ERROR_FEEDBACK", "1"),
    ("(gint) GST_AUDIO_NOISE_SHAPING_HIGH", "4"),
    ("(gint) GST_AUDIO_NOISE_SHAPING_MEDIUM", "3"),
    ("(gint) GST_AUDIO_NOISE_SHAPING_NONE", "0"),
    ("(gint) GST_AUDIO_NOISE_SHAPING_SIMPLE", "2"),
    ("(guint) GST_AUDIO_PACK_FLAG_NONE", "0"),
    ("(guint) GST_AUDIO_PACK_FLAG_TRUNCATE_RANGE", "1"),
    ("(guint) GST_AUDIO_QUANTIZE_FLAG_NONE", "0"),
    ("(guint) GST_AUDIO_QUANTIZE_FLAG_NON_INTERLEAVED", "1"),
    ("GST_AUDIO_RATE_RANGE", "(int) [ 1, max ]"),
    ("(gint) GST_AUDIO_RESAMPLER_FILTER_INTERPOLATION_CUBIC", "2"),
    (
        "(gint) GST_AUDIO_RESAMPLER_FILTER_INTERPOLATION_LINEAR",
        "1",
    ),
    ("(gint) GST_AUDIO_RESAMPLER_FILTER_INTERPOLATION_NONE", "0"),
    ("(gint) GST_AUDIO_RESAMPLER_FILTER_MODE_AUTO", "2"),
    ("(gint) GST_AUDIO_RESAMPLER_FILTER_MODE_FULL", "1"),
    ("(gint) GST_AUDIO_RESAMPLER_FILTER_MODE_INTERPOLATED", "0"),
    ("(guint) GST_AUDIO_RESAMPLER_FLAG_NONE", "0"),
    ("(guint) GST_AUDIO_RESAMPLER_FLAG_NON_INTERLEAVED_IN", "1"),
    ("(guint) GST_AUDIO_RESAMPLER_FLAG_NON_INTERLEAVED_OUT", "2"),
    ("(guint) GST_AUDIO_RESAMPLER_FLAG_VARIABLE_RATE", "4"),
    ("(gint) GST_AUDIO_RESAMPLER_METHOD_BLACKMAN_NUTTALL", "3"),
    ("(gint) GST_AUDIO_RESAMPLER_METHOD_CUBIC", "2"),
    ("(gint) GST_AUDIO_RESAMPLER_METHOD_KAISER", "4"),
    ("(gint) GST_AUDIO_RESAMPLER_METHOD_LINEAR", "1"),
    ("(gint) GST_AUDIO_RESAMPLER_METHOD_NEAREST", "0"),
    (
        "GST_AUDIO_RESAMPLER_OPT_CUBIC_B",
        "GstAudioResampler.cubic-b",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_CUBIC_C",
        "GstAudioResampler.cubic-c",
    ),
    ("GST_AUDIO_RESAMPLER_OPT_CUTOFF", "GstAudioResampler.cutoff"),
    (
        "GST_AUDIO_RESAMPLER_OPT_FILTER_INTERPOLATION",
        "GstAudioResampler.filter-interpolation",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_FILTER_MODE",
        "GstAudioResampler.filter-mode",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_FILTER_MODE_THRESHOLD",
        "GstAudioResampler.filter-mode-threshold",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_FILTER_OVERSAMPLE",
        "GstAudioResampler.filter-oversample",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_MAX_PHASE_ERROR",
        "GstAudioResampler.max-phase-error",
    ),
    ("GST_AUDIO_RESAMPLER_OPT_N_TAPS", "GstAudioResampler.n-taps"),
    (
        "GST_AUDIO_RESAMPLER_OPT_STOP_ATTENUATION",
        "GstAudioResampler.stop-attenutation",
    ),
    (
        "GST_AUDIO_RESAMPLER_OPT_TRANSITION_BANDWIDTH",
        "GstAudioResampler.transition-bandwidth",
    ),
    ("GST_AUDIO_RESAMPLER_QUALITY_DEFAULT", "4"),
    ("GST_AUDIO_RESAMPLER_QUALITY_MAX", "10"),
    ("GST_AUDIO_RESAMPLER_QUALITY_MIN", "0"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_AC3", "7"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_A_LAW", "2"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_DTS", "9"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_EAC3", "8"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_FLAC", "14"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_GSM", "5"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_IEC958", "6"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_IMA_ADPCM", "3"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MPEG", "4"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MPEG2_AAC", "10"),
    (
        "(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MPEG2_AAC_RAW",
        "12",
    ),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MPEG4_AAC", "11"),
    (
        "(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MPEG4_AAC_RAW",
        "13",
    ),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_MU_LAW", "1"),
    ("(gint) GST_AUDIO_RING_BUFFER_FORMAT_TYPE_RAW", "0"),
    ("(gint) GST_AUDIO_RING_BUFFER_STATE_ERROR", "3"),
    ("(gint) GST_AUDIO_RING_BUFFER_STATE_PAUSED", "1"),
    ("(gint) GST_AUDIO_RING_BUFFER_STATE_STARTED", "2"),
    ("(gint) GST_AUDIO_RING_BUFFER_STATE_STOPPED", "0"),
    ("GST_META_TAG_AUDIO_CHANNELS_STR", "channels"),
    ("GST_META_TAG_AUDIO_RATE_STR", "rate"),
    ("GST_META_TAG_AUDIO_STR", "audio"),
    ("(gint) GST_STREAM_VOLUME_FORMAT_CUBIC", "1"),
    ("(gint) GST_STREAM_VOLUME_FORMAT_DB", "2"),
    ("(gint) GST_STREAM_VOLUME_FORMAT_LINEAR", "0"),
];
