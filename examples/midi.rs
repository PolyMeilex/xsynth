use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, HostTrait};
use midi_toolkit::{
    events::{Event, MIDIEvent},
    io::MIDIFile,
    pipe,
    sequence::{
        event::{cancel_tempo_events, merge_events_array, scale_event_time},
        to_vec, unwrap_items, TimeCaster,
    },
};
use xsynth::{
    core::{
        event::{ChannelEvent, ControlEvent},
        soundfont::{SoundfontBase, SquareSoundfont},
    },
    RealtimeSynth, SynthEvent,
};

fn main() {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find output device");
    println!("Output device: {}", device.name().unwrap());

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    let synth = RealtimeSynth::new(16, &device, config);
    let mut sender = synth.get_senders();

    let params = synth.stream_params();
    let soundfonts: Vec<Arc<dyn SoundfontBase>> = vec![Arc::new(SquareSoundfont::new(
        params.sample_rate,
        params.channels,
    ))];

    sender.send_event(SynthEvent::AllChannels(ChannelEvent::SetSoundfonts(
        soundfonts,
    )));

    let midi = MIDIFile::open(
        "D:/Midis/[Black MIDI]scarlet_zone-& The Young Descendant of Tepes V.2.mid",
        None,
    )
    .unwrap();
    let ppq = midi.ppq();
    let merged = pipe!(
        midi.iter_all_tracks()
        |>to_vec()
        |>merge_events_array()
        |>TimeCaster::<f64>::cast_event_delta()
        |>cancel_tempo_events(250000)
        |>scale_event_time(1.0 / ppq as f64)
        |>unwrap_items()
    );

    let (tx, rx) = crossbeam_channel::bounded(16777216);

    thread::spawn(move || {
        for e in merged {
            tx.send(e).unwrap();
        }
    });

    let now = Instant::now();
    let mut time = 0.0;
    for e in rx.iter() {
        if e.delta() != 0.0 {
            time += e.delta();
            let diff = time - now.elapsed().as_secs_f64();
            if diff > 0.0 {
                spin_sleep::sleep(Duration::from_secs_f64(diff));
            }
        }

        match e {
            Event::NoteOn(e) => {
                sender.send_event(SynthEvent::Channel(
                    e.channel as u32,
                    ChannelEvent::NoteOn {
                        key: e.key,
                        vel: e.velocity,
                    },
                ));
            }
            Event::NoteOff(e) => {
                sender.send_event(SynthEvent::Channel(
                    e.channel as u32,
                    ChannelEvent::NoteOff { key: e.key },
                ));
            }
            Event::ControlChange(e) => {
                sender.send_event(SynthEvent::Channel(
                    e.channel as u32,
                    ChannelEvent::Control(ControlEvent::Raw(e.controller, e.value)),
                ));
            }
            Event::PitchWheelChange(e) => {
                sender.send_event(SynthEvent::Channel(
                    e.channel as u32,
                    ChannelEvent::Control(ControlEvent::PitchBendValue(e.pitch as f32 / 8_192.0)),
                ));
            }
            _ => {}
        }
    }

    std::thread::sleep(Duration::from_secs(10000));
}
