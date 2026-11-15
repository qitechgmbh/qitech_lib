//! qi-heating PoC — Terminal Edition (nach dem Vorbild der QiTech Control TUI)
//!
//! Bus-Scan + Live-View aller Klemmen:
//! - EL3202/EL3204: Measurements — T1..T4 in °C inkl. Offen-/Kurzschluss-Erkennung
//! - EL1814/EL1804: State — DI1..DI4
//! - EL2808/EL2634: Commands — Ausgänge per Tasten 1-8 togglen
//! - EL3062:   Measurements — AI1/AI2 in V (0..10 V, Öltank-Füllstand)
//! - EL2502:   Commands — PWM-Duty in % (Tasten -/= Kanal 1, ,/. Kanal 2, ±5 %)
//!
//! Tasten: ↑/↓ Maschine wählen · q/Esc beenden
//! Start:  sudo -E ./heating_tui <iface>     (Linux: ETHERCAT_HAL_NO_IO_URING=1)

use bitvec::{order::Lsb0, slice::BitSlice};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState, MetaSubdevice,
    coe::Configuration,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::{
            el1804::{EL1804, EL1804_PRODUCT_ID},
            el1814::{EL1814, EL1814_PRODUCT_ID},
            el2502::{EL2502, EL2502_PRODUCT_ID},
            el2634::{EL2634, EL2634_PRODUCT_ID},
            el2808::{EL2808, EL2808_PRODUCT_ID},
            el3062::{EL3062, EL3062Configuration, EL3062_PRODUCT_ID},
            el3202::{EL3202, EL3202Configuration, EL3202_PRODUCT_ID},
            el3204::{EL3204, EL3204Configuration, EL3204_PRODUCT_ID},
        },
    },
    init_ethercat,
    io::analog_input::AnalogInputDevice,
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
    Rtd(RtdKind),
    Di(DiKind),
    Do(DoKind),
    Ai(EL3062),
    Pwm(EL2502),
}

enum RtdKind {
    El3202(EL3202),
    El3204(EL3204),
}

impl RtdKind {
    fn get(&self, port: usize) -> Option<ethercat_hal::io::temperature_input::TemperatureInputInput> {
        match self {
            RtdKind::El3202(el) => el.get_input(port).ok(),
            RtdKind::El3204(el) => el.get_input(port).ok(),
        }
    }

    fn port_count(&self) -> usize {
        match self {
            RtdKind::El3202(el) => el.get_port_count(),
            RtdKind::El3204(el) => el.get_port_count(),
        }
    }

    fn input_from(&mut self, slice: &BitSlice<u8, Lsb0>) {
        match self {
            RtdKind::El3202(el) => {
                el.input(slice).ok();
            }
            RtdKind::El3204(el) => {
                el.input(slice).ok();
            }
        }
    }
}

enum DoKind {
    El2808(EL2808),
    El2634(EL2634),
}

impl DoKind {
    fn get(&self, port: usize) -> bool {
        match self {
            DoKind::El2808(el) => el.get_output_state(port),
            DoKind::El2634(el) => el.get_output_state(port),
        }
    }

    fn set(&mut self, port: usize, value: bool) {
        match self {
            DoKind::El2808(el) => el.set_output(port, value),
            DoKind::El2634(el) => el.set_output(port, value),
        }
    }

    fn port_count(&self) -> usize {
        match self {
            DoKind::El2808(el) => el.get_port_count(),
            DoKind::El2634(el) => el.get_port_count(),
        }
    }

    fn output_to(&mut self, slice: &mut BitSlice<u8, Lsb0>) {
        match self {
            DoKind::El2808(el) => {
                el.output(slice).ok();
            }
            DoKind::El2634(el) => {
                el.output(slice).ok();
            }
        }
    }
}

enum DiKind {
    El1804(EL1804),
    El1814(EL1814),
}

impl DiKind {
    fn get(&self, port: usize) -> bool {
        match self {
            DiKind::El1804(el) => el.get_input(port).unwrap_or(false),
            DiKind::El1814(el) => el.get_input(port).unwrap_or(false),
        }
    }

    fn input_from(&mut self, slice: &BitSlice<u8, Lsb0>) {
        match self {
            DiKind::El1804(el) => {
                el.input(slice).ok();
            }
            DiKind::El1814(el) => {
                el.input(slice).ok();
            }
        }
    }
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

impl App {
    /// Wählt den zum Gerätetyp passenden Tab automatisch
    /// (RTD→Measurements, DI→State, DO→Commands), damit jede Maschine sofort
    /// ihren sinnvollen Inhalt zeigt.
    fn sync_tab_to_kind(&mut self) {
        self.tab = match self.devices.get(self.selected).map(|d| &d.kind) {
            Some(DevKind::Rtd(_)) | Some(DevKind::Ai(_)) => Tab::Measurements,
            Some(DevKind::Di(_)) => Tab::State,
            Some(DevKind::Do(_)) | Some(DevKind::Pwm(_)) => Tab::Commands,
            None => self.tab,
        };
    }
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

    // WICHTIG: SDO/CoE-Requests werden vom HAL nur im PreOp bedient
    // (preop_group) — Konfiguration muss VOR dem Op-Übergang geschehen.
    // ACHTUNG: In PreOp haben die Subdevices noch keine PDO-Offsets
    // (start_tx==end_tx) — die verbindliche Liste wird nach Op erneut geholt.
    let subdevices_preop = eth_handle
        .try_get_subdevices_vec_sync()
        .expect("could not list subdevices");

    // EL3202-Sensortyp auf PT1000 stellen (Anlage ist durchgehend PT1000;
    // Beckhoff-Auslieferungszustand ist PT100 → 1-kΩ-Fühler würde sonst als
    // „offen/Overrange" angezeigt)
    for meta in &subdevices_preop {
        if meta.product_id == EL3202_PRODUCT_ID {
            EL3202Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("PT1000-Konfiguration fehlgeschlagen");
            println!("EL3202 @0x{:x}: Sensortyp = PT1000", meta.device_address);
        }
        // EL3204: gleiche PT1000-Umstellung, 4 Kanäle (0x8000..0x8030)
        if meta.product_id == EL3204_PRODUCT_ID {
            EL3204Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("EL3204 PT1000-Konfiguration fehlgeschlagen");
            println!(
                "EL3204 @0x{:x}: Sensortyp = PT1000 (4 Kanäle)",
                meta.device_address
            );
        }
        // EL3062 (Öltank): Default-Config ≈ Werkszustand schreiben — die
        // gebrauchte Klemme hatte persistierte CoE-Settings des Vorbesitzers
        // (AL 0x001E „Invalid Input Configuration" bei SafeOp, 2026-11-15).
        if meta.product_id == EL3062_PRODUCT_ID {
            EL3062Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("EL3062-Konfiguration fehlgeschlagen");
            println!("EL3062 @0x{:x}: Default-Config (0..10 V)", meta.device_address);
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");
    while !matches!(eth_handle.get_state(), EtherCATState::Op) {
        std::thread::sleep(Duration::from_millis(10));
    }

    // Nach Op: PDO-Offsets vergeben → verbindliche Geräteliste
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
            EL3202_PRODUCT_ID => Some(DevKind::Rtd(RtdKind::El3202(EL3202::new()))),
            EL3204_PRODUCT_ID => Some(DevKind::Rtd(RtdKind::El3204(EL3204::new()))),
            EL1804_PRODUCT_ID => Some(DevKind::Di(DiKind::El1804(EL1804::new()))),
            EL1814_PRODUCT_ID => Some(DevKind::Di(DiKind::El1814(EL1814::new()))),
            EL2808_PRODUCT_ID => Some(DevKind::Do(DoKind::El2808(EL2808::new()))),
            EL2634_PRODUCT_ID => Some(DevKind::Do(DoKind::El2634(EL2634::new()))),
            EL3062_PRODUCT_ID => Some(DevKind::Ai(EL3062::new())),
            EL2502_PRODUCT_ID => Some(DevKind::Pwm(EL2502::new())),
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
                        app.selected = app.selected.saturating_sub(1);
                        app.sync_tab_to_kind();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected = (app.selected + 1).min(app.devices.len().saturating_sub(1));
                        app.sync_tab_to_kind();
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
                            if let DevKind::Do(kind) = &mut dev.kind {
                                if n < kind.port_count() {
                                    let cur = kind.get(n);
                                    kind.set(n, !cur);
                                }
                            }
                        }
                    }
                    // EL2502 PWM-Duty: '-'/ '=' für Kanal 1, ','/'.' für Kanal 2 (±5 %)
                    KeyCode::Char('-') | KeyCode::Char('=') | KeyCode::Char(',')
                    | KeyCode::Char('.') => {
                        if let Some(dev) = app.devices.get_mut(app.selected) {
                            if let DevKind::Pwm(el) = &mut dev.kind {
                                let (ch, delta) = match key.code {
                                    KeyCode::Char('-') => (0, -5.0),
                                    KeyCode::Char('=') => (0, 5.0),
                                    KeyCode::Char(',') => (1, -5.0),
                                    KeyCode::Char('.') => (1, 5.0),
                                    _ => unreachable!(),
                                };
                                let cur = el.duty_raw(ch).unwrap_or(0) as f64 / 100.0;
                                el.set_duty_percent(ch, (cur + delta).clamp(0.0, 100.0));
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
                    DevKind::Rtd(kind) => {
                        kind.input_from(BitSlice::<u8, Lsb0>::from_slice(&inputs[s..e]));
                    }
                    DevKind::Di(el) => {
                        el.input_from(BitSlice::<u8, Lsb0>::from_slice(&inputs[s..e]));
                    }
                    DevKind::Ai(el) => {
                        el.input(BitSlice::<u8, Lsb0>::from_slice(&inputs[s..e])).ok();
                    }
                    DevKind::Do(_) | DevKind::Pwm(_) => (),
                }
            }
        }
    }

    if let Some(outputs) = eth_handle.write_outputs() {
        for dev in app.devices.iter_mut() {
            let (s, e) = (dev.meta.start_rx, dev.meta.end_rx);
            if e > s {
                match &mut dev.kind {
                    DevKind::Do(kind) => {
                        kind.output_to(BitSlice::<u8, Lsb0>::from_slice_mut(&mut outputs[s..e]));
                    }
                    DevKind::Pwm(el) => {
                        el.output(BitSlice::<u8, Lsb0>::from_slice_mut(&mut outputs[s..e]))
                            .ok();
                    }
                    _ => (),
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
        // Inhalt folgt immer dem Gerätetyp; der Tab ist zur Info mitgeschaltet
        Some(dev) => detail_content(dev, app.tab),
        None => vec![Line::from("(keine Maschine)")],
    };
    f.render_widget(
        Paragraph::new(body).block(Block::default().borders(Borders::ALL)),
        detail_chunks[1],
    );
}

fn detail_content(dev: &mut BusDevice, _tab: Tab) -> Vec<Line<'static>> {
    // Wir zeigen je Gerätetyp das passende Panel — auch wenn ein „fremder" Tab
    // angewählt ist (vor der Umstellung wirkten DI/DO wie „kaputt").
    let tab = match dev.kind {
        DevKind::Rtd(_) | DevKind::Ai(_) => Tab::Measurements,
        DevKind::Di(_) => Tab::State,
        DevKind::Do(_) | DevKind::Pwm(_) => Tab::Commands,
    };
    match (&mut dev.kind, tab) {
        (DevKind::Rtd(kind), Tab::Measurements) => (0..kind.port_count())
            .map(|port| match kind.get(port) {
                Some(input) => {
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
                None => Line::from(format!("T{}: <Lesefehler>", port + 1)),
            })
            .collect(),
        (DevKind::Rtd(_), _) => vec![Line::from("(s. Measurements)")],
        (DevKind::Ai(el), Tab::Measurements) => (0..el.get_port_count())
            .map(|port| match el.get_input(port) {
                Ok(input) => {
                    // EL3062: Messbereich 0..10 V (Öltank-Füllstand)
                    let volts = f64::from(input.normalized) * 10.0;
                    Line::from(vec![
                        Span::raw(format!("AI{}:     ", port + 1)),
                        Span::styled(
                            format!("{:.2} V ({:.0} %)", volts, input.normalized * 100.0),
                            Style::default().fg(Color::Green),
                        ),
                    ])
                }
                Err(e) => Line::from(format!("AI{}: {:?}", port + 1, e)),
            })
            .collect(),
        (DevKind::Ai(_), _) => vec![Line::from("(s. Measurements)")],
        (DevKind::Di(el), Tab::State) => (0..4)
            .map(|port| {
                let on = el.get(port);
                Line::from(format!("DI{}:  {}", port + 1, if on { "■" } else { "□" }))
            })
            .collect(),
        (DevKind::Di(_), _) => vec![Line::from("(s. State)")],
        (DevKind::Do(kind), Tab::Commands) => (0..kind.port_count())
            .map(|port| {
                let on = kind.get(port);
                Line::from(format!(
                    "out{}:  {}   [Taste {} toggelt]",
                    port + 1,
                    if on { "true" } else { "false" },
                    port + 1
                ))
            })
            .collect(),
        (DevKind::Do(_), _) => vec![Line::from("(s. Commands)")],
        (DevKind::Pwm(el), Tab::Commands) => {
            let mut lines: Vec<Line> = (0..el.get_port_count())
                .map(|port| {
                    let percent = el.duty_raw(port).unwrap_or(0) as f64 / 100.0;
                    let keys = if port == 0 { "-/=" } else { ",/." };
                    Line::from(format!(
                        "PWM{}:  {:5.1} %   [{} ±5 %]",
                        port + 1,
                        percent,
                        keys
                    ))
                })
                .collect();
            lines.push(Line::from(Span::styled(
                "Duty wirkt sofort im nächsten Zyklus. Pumpen-Test: 30 % ≈ Mindestlauf.",
                Style::default().fg(Color::DarkGray),
            )));
            lines
        }
        (DevKind::Pwm(_), _) => vec![Line::from("(s. Commands)")],
    }
}
