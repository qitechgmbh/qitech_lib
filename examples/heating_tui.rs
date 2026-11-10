//! qi-heating PoC — Terminal Edition (nach dem Vorbild der QiTech Control TUI)
//!
//! Bus-Scan + Live-View aller Bestandsklemmen:
//! - EL3202:   Measurements — T1/T2 in °C inkl. Offen-/Kurzschluss-Erkennung
//! - EL1814:   State — DI1..DI8
//! - EL2808:   Commands — DO1..DO8 per Tasten 1-8 togglen
//!
//! Tasten: ↑/↓ Maschine wählen · 1-8 Ausgang togglen (EL2808) · q/Esc beenden
//! Start:  sudo -E ./heating_tui <iface>     (Linux: ETHERCAT_HAL_NO_IO_URING=1)

use bitvec::{order::Lsb0, slice::BitSlice};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState, MetaSubdevice,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::{
            el1814::{EL1814, EL1814_PRODUCT_ID},
            el2808::{EL2808, EL2808_PRODUCT_ID},
            el3202::{EL3202, EL3202_PRODUCT_ID},
        },
    },
    init_ethercat,
    io::digital_input::DigitalInputDevice,
    io::digital_output::DigitalOutputDevice,
    io::temperature_input::TemperatureInputDevice,
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use std::{env, io, time::Duration};

enum DevKind {
    Rtd(EL3202),
    Di(EL1814),
    Do(EL2808),
}

struct BusDevice {
    meta: MetaSubdevice,
    kind: DevKind,
    label: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Measurements,
    State,
    Commands,
}

struct App {
    devices: Vec<BusDevice>,
    selected: usize,
    tab: Tab,
    ecat_state: EtherCATState,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let interface = env::args().nth(1).expect("Usage: heating_tui <interface>");

    let eth_control = init_ethercat(&interface, None);
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    while !matches!(eth_handle.get_state(), EtherCATState::PreOp) {
        std::thread::sleep(Duration::from_millis(10));
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");
    while !matches!(eth_handle.get_state(), EtherCATState::Op) {
        std::thread::sleep(Duration::from_millis(10));
    }

    let subdevices = eth_handle
        .try_get_subdevices_vec_sync()
        .expect("could not list subdevices");

    let mut devices: Vec<BusDevice> = Vec::new();
    for (i, meta) in subdevices.iter().enumerate() {
        if meta.vendor != BECKHOFF_VENDOR_ID {
            continue;
        }
        let name = meta.get_name().unwrap_or_else(|_| "?".to_string());
        let kind = match meta.product_id {
            EL3202_PRODUCT_ID => Some(DevKind::Rtd(EL3202::new())),
            EL1814_PRODUCT_ID => Some(DevKind::Di(EL1814::new())),
            EL2808_PRODUCT_ID => Some(DevKind::Do(EL2808::new())),
            _ => None,
        };
        if let Some(kind) = kind {
            devices.push(BusDevice {
                meta: *meta,
                kind,
                label: format!("{} ({})", name, i),
            });
        }
    }

    let mut app = App {
        devices,
        selected: 0,
        tab: Tab::Measurements,
        ecat_state: EtherCATState::Op,
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        app_tick(&mut app, &mut eth_handle);
        terminal.draw(|f| draw(f, &mut app))?;

        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.selected = app.selected.saturating_sub(1)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected = (app.selected + 1).min(app.devices.len().saturating_sub(1))
                    }
                    KeyCode::Tab => {
                        app.tab = match app.tab {
                            Tab::Measurements => Tab::State,
                            Tab::State => Tab::Commands,
                            Tab::Commands => Tab::Measurements,
                        }
                    }
                    KeyCode::Char(c @ '1'..='8') => {
                        let n = c.to_digit(10).unwrap() as usize - 1;
                        if let Some(dev) = app.devices.get_mut(app.selected) {
                            if let DevKind::Do(el) = &mut dev.kind {
                                let cur = el.get_output_state(n);
                                el.set_output(n, !cur);
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Ein Zyklus: PDOs einlesen/schreiben
fn app_tick(app: &mut App, eth_handle: &mut ethercat_hal::StdEcatHandle) {
    app.ecat_state = eth_handle.get_state();

    if let Some(inputs) = eth_handle.get_inputs() {
        for dev in app.devices.iter_mut() {
            let (s, e) = (dev.meta.start_tx, dev.meta.end_tx);
            if e > s {
                match &mut dev.kind {
                    DevKind::Rtd(el) => {
                        el.input(BitSlice::<u8, Lsb0>::from_slice(&inputs[s..e])).ok();
                    }
                    DevKind::Di(el) => {
                        el.input(BitSlice::<u8, Lsb0>::from_slice(&inputs[s..e])).ok();
                    }
                    DevKind::Do(_) => (),
                }
            }
        }
    }

    if let Some(outputs) = eth_handle.write_outputs() {
        for dev in app.devices.iter_mut() {
            let (s, e) = (dev.meta.start_rx, dev.meta.end_rx);
            if e > s {
                if let DevKind::Do(el) = &mut dev.kind {
                    el.output(BitSlice::<u8, Lsb0>::from_slice_mut(&mut outputs[s..e]))
                        .ok();
                }
            }
        }
    }
    eth_handle.send_outputs();
}

fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(9),
            Constraint::Min(8),
        ])
        .split(f.area());

    // --- Status ---
    let status = vec![
        Line::from("Status:"),
        Line::from(vec![
            Span::raw("Runtime:  "),
            Span::styled("● Running", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("EtherCAT: "),
            Span::styled(
                format!("● {:?}", app.ecat_state),
                Style::default().fg(Color::Green),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(status).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" qi-heating PoC (Terminal Edition) "),
        ),
        chunks[0],
    );

    // --- Machines ---
    let items: Vec<ListItem> = app
        .devices
        .iter()
        .map(|d| ListItem::new(d.label.clone()))
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Machines · (↑/↓ select) "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(list, chunks[1], &mut state);

    // --- Details ---
    let titles = ["Measurements", "State", "Commands"];
    let tab_idx = match app.tab {
        Tab::Measurements => 0,
        Tab::State => 1,
        Tab::Commands => 2,
    };
    let tabs = Tabs::new(titles.iter().map(|t| format!(" {} ", t)).collect::<Vec<_>>())
        .select(tab_idx)
        .highlight_style(Style::default().fg(Color::Yellow));
    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(4)])
        .split(chunks[2]);
    f.render_widget(tabs, detail_chunks[0]);

    let body = match app.devices.get_mut(app.selected) {
        Some(dev) => detail_content(dev, app.tab),
        None => vec![Line::from("(keine Maschine)")],
    };
    f.render_widget(
        Paragraph::new(body).block(Block::default().borders(Borders::ALL)),
        detail_chunks[1],
    );
}

fn detail_content(dev: &mut BusDevice, tab: Tab) -> Vec<Line<'static>> {
    match (&mut dev.kind, tab) {
        (DevKind::Rtd(el), Tab::Measurements) => (0..el.get_port_count())
            .map(|port| match el.get_input(port) {
                Ok(input) => {
                    if input.error || input.overvoltage {
                        Line::from(vec![
                            Span::raw(format!("T{}:      ", port + 1)),
                            Span::styled("< offen / kein Fühler >", Style::default().fg(Color::Red)),
                        ])
                    } else if input.undervoltage {
                        Line::from(vec![
                            Span::raw(format!("T{}:      ", port + 1)),
                            Span::styled("< Kurzschluss >", Style::default().fg(Color::Magenta)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw(format!("T{}:      ", port + 1)),
                            Span::styled(
                                format!("{:.1} °C", input.temperature),
                                Style::default().fg(Color::Green),
                            ),
                        ])
                    }
                }
                Err(e) => Line::from(format!("T{}: {:?}", port + 1, e)),
            })
            .collect(),
        (DevKind::Rtd(_), _) => vec![Line::from("(Tab-State/Commands nicht verwendet)")],
        (DevKind::Di(el), Tab::State) => (0..el.get_port_count())
            .map(|port| {
                let on = el.get_input(port).unwrap_or(false);
                Line::from(format!("DI{}:  {}", port + 1, if on { "■" } else { "□" }))
            })
            .collect(),
        (DevKind::Di(_), _) => vec![Line::from("(Tab Measurements/Commands nicht verwendet)")],
        (DevKind::Do(el), Tab::Commands) => (0..el.get_port_count())
            .map(|port| {
                let on = el.get_output_state(port);
                Line::from(format!(
                    "leds_on{}:  {}   [Taste {} toggelt]",
                    port + 1,
                    if on { "true" } else { "false" },
                    port + 1
                ))
            })
            .collect(),
        (DevKind::Do(_), _) => vec![Line::from("(Tab Measurements/State nicht verwendet)")],
    }
}
