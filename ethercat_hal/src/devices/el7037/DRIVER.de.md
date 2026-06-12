# EL7037 — Treiber-Dokumentation

Diese Dokumentation erklärt am konkreten Beispiel der Beckhoff EL7037, wie ein
EtherCAT-Geräterieber in `ethercat_hal` aufgebaut ist, welche Designentscheidungen
dahinterstehen und wie man nach diesem Muster weitere EL70xx-Klemmen implementiert.

---

## 1. Überblick & Hardware

Die **EL7037** ist eine kompakte Schrittmotor-Endklemme von Beckhoff:

- Versorgungsspannung: 24 V DC
- Ausgangsstrom: bis 1,5 A (Spitzenstrom)
- Integrierter inkrementeller Encoder-Eingang (5 V)
- Betriebsmodi: **Velocity-Modus** (Direktgeschwindigkeit), **Position-Modus**
  (über integrierte Positioniereinheit)
- 2 digitale Eingänge (z. B. für Endschalter), direkt in `StmStatus` ablesbar

### Offizielle Beckhoff-Dokumentation

| Ressource | Link |
|---|---|
| Produktseite | <https://www.beckhoff.com/de-de/produkte/i-o/ethercat-klemmen/el-ed-elm7xxx-kompakte-antriebstechnik/el7037.html> |
| Technische Dokumentation (PDF) | <https://www.beckhoff.com/de-de/download/50323079> |

> **Hinweis zur Variantenwahl der shared_config:**
> Die EL7037 verwendet `shared_config::el70x7` — **nicht** `shared_config::el70x1`
> wie die Geschwisterklemmen EL7031/EL7041. Die wesentlichen Unterschiede:
> - `StmControllerConfiguration` (0x8011) hat bei der EL7037 nur zwei Subindizes
>   (`kp_factor`, `ki_factor`); EL7031/EL7041 haben sieben.
> - `StmFeatures` muss `operation_mode` (0x8012:01) **explizit** schreiben; beim
>   EL7031 geschieht das implizit.
> - Die Info-Daten-Enums heißen `EL70x7InfoData` und bieten andere Werte
>   (`MotorLoad`, `MotorDcCurrent`, …).

---

## 2. Dateistruktur des Treibers

```
ethercat_hal/src/devices/el7037/
├── mod.rs       — Gerätestruct, Trait-Implementierungen, Identitätskonstanten
├── coe.rs       — Azyklische Konfiguration (CoE-SDO-Writes vor dem Betrieb)
└── pdo.rs       — Zyklische Prozessdaten (PDO-Objekte, Betriebsmodus-Auswahl)
```

Diese Dreiteilung ist das **Standardmuster** aller EL70xx-Treiber im Crate.
Jede Datei hat genau eine Verantwortlichkeit:

| Datei | Zeitpunkt | Protokoll |
|---|---|---|
| `coe.rs` | Einmalig beim Start (PreOp → SafeOp) | SDO (azyklisch) |
| `pdo.rs` | Zyklisch im Op-Modus (jede EtherCAT-Periode) | PDO (zyklisch) |
| `mod.rs` | Laufzeit-Logik, verbindet coe + pdo | — |

---

## 3. Identität & Registrierung

### Warum eine Identität nötig ist

EtherCAT erkennt Teilnehmer ausschließlich über drei 32-Bit-Werte aus der
Klemmen-EEPROM: **Vendor ID**, **Product Code** und **Revision**. Ohne diese
Zuordnung kann der Stack kein Geräteobjekt erzeugen.

### Konstanten in `mod.rs`

```rust
// mod.rs, Zeilen 340–344
pub const EL7037_VENDOR_ID:  u32 = 0x2;
pub const EL7037_PRODUCT_ID: u32 = 0x1b7d3052;
pub const EL7037_REVISION_A: u32 = 0x00170000;

pub const EL7037_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7037_VENDOR_ID, EL7037_PRODUCT_ID, EL7037_REVISION_A);
```

`SubDeviceIdentityTuple` ist ein Typ-Alias `(u32, u32, u32)` aus
`devices/mod.rs`.

### Registrierung in `devices/mod.rs`

Drei Stellen müssen angepasst werden, damit eine neue Klemme im System sichtbar
wird:

```rust
// 1. Modul deklarieren
pub mod el7037;

// 2. Identitätskonstante importieren
use el7037::EL7037_IDENTITY_A;

// 3a. Box-Variante (für den normalen Controller-Pfad)
fn device_from_subdevice_identity(dev: &SubDeviceInfo) -> Result<Box<dyn EthercatDevice>> {
    match (dev.vendor, dev.product_id, dev.revision) {
        EL7037_IDENTITY_A => Ok(Box::new(el7037::EL7037::new())),
        // …
    }
}

// 3b. Rc<RefCell>-Variante (für den Sharing-Pfad, gleiche Struktur)
```

### Identitätswerte ermitteln

Die Werte lassen sich per EEPROM-Dump aus einer angeschlossenen Klemme auslesen.
Dafür steht das Skill `ethercat-eeprom-dump` zur Verfügung (Skill-Datei:
`.claude/skills/ethercat-eeprom-dump/SKILL.md` im `control`-Repo).

---

## 4. Zyklische Prozessdaten (PDOs)

### Grundprinzip

Im Op-Modus tauscht EtherCAT in jeder Periode (typisch 1 ms) einen fixen
Datenpuffer aus:

- **TxPDO** (Klemme → Controller): Sensorwerte, Status-Bits
- **RxPDO** (Controller → Klemme): Sollwerte, Steuerbits

Welche Objekte in diesem Puffer enthalten sind, wird einmalig beim Start über
**PDO-Assignment-Register** (0x1C13 / 0x1C12) per SDO konfiguriert.

### PDO-Structs (`pdo.rs`)

```rust
#[derive(Debug, Clone, TxPdo)]
pub struct EL7037TxPdo {
    #[pdo_object_index(0x1A00)]
    pub enc_status_compact: Option<EncStatusCompact>,  // Encoder-Zähler, kompakt

    #[pdo_object_index(0x1A03)]
    pub stm_status: Option<StmStatus>,                 // Motor-Statusbits

    // … weitere optionale PDO-Objekte
}
```

Jedes Feld ist `Option<T>`:
- `Some(T)` → dieses PDO-Objekt ist **aktiv** und wird im Frame übertragen.
- `None` → inaktiv, nimmt keinen Platz im Frame ein.

Das **Derive-Makro `TxPdo`** (aus `ethercat_hal_derive`) generiert daraus
automatisch:
1. `impl Configuration` → schreibt die aktiven PDO-Indizes in `0x1C13` (TxPDO)
   bzw. `0x1C12` (RxPDO) per SDO.
2. `impl TxPdo` / `impl RxPdo` → liefert die PDO-Objekte als Slices, damit der
   Controller sie serialisieren / deserialisieren kann.

### PDO-Objekte

Jedes PDO-Objekt (z. B. `StmControl`) ist ein einfaches Struct:

```rust
// pdo/el70x7.rs
#[derive(Debug, Clone, Default, PdoObject)]
#[pdo_object(bits = 16)]       // Gesamtgröße: 2 Bytes
pub struct StmControl {
    pub enable: bool,           // Bit 0  (CoE 7010:01)
    pub reset: bool,            // Bit 1  (CoE 7010:02)
    pub reduce_torque: bool,    // Bit 2  (CoE 7010:03)
}

impl RxPdoObject for StmControl {
    fn write(&self, buffer: &mut BitSlice<u8, Lsb0>) {
        buffer.set(0, self.enable);
        buffer.set(1, self.reset);
        buffer.set(2, self.reduce_torque);
    }
}
```

Das Bit-Mapping (`bitvec`) wird von Hand implementiert und muss exakt zum
Beckhoff-Datenblatt passen. Das Attribut `#[pdo_object(bits = N)]` gibt die
Gesamtbitbreite an, die `TxPdo`/`RxPdo` für die Puffer-Größenberechnung nutzt.

### Betriebsmodus-Auswahl: `EL7037PredefinedPdoAssignment`

Beckhoff definiert für jede Klemme eine feste Menge erlaubter PDO-Kombinationen.
Diese sind als Enum codiert:

```rust
pub enum EL7037PredefinedPdoAssignment {
    #[default]
    VelocityControlCompact,           // Standard: enc_status_compact + stm_status | enc_ctrl_compact + stm_ctrl + stm_vel
    VelocityControlCompactWithInfoData,
    VelocityControl,
    PositionControl,
    PositionInterfaceCompact,
    PositionInterface,
    // …
}
```

Der `PredefinedPdoAssignment`-Trait definiert für jeden Enum-Zweig, welche
Felder `Some` sind:

```rust
impl PredefinedPdoAssignment<EL7037TxPdo, EL7037RxPdo> for EL7037PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL7037TxPdo { /* … */ }
    fn rxpdo_assignment(&self) -> EL7037RxPdo { /* … */ }
}
```

Die Unit-Tests in `pdo.rs` prüfen die exakten Byte-Größen (z. B.
`VelocityControlCompact` = 8 Byte Tx + 8 Byte Rx), damit ein versehentlicher
Konfigurationsfehler sofort auffällt.

---

## 5. Azyklische Konfiguration (CoE / `coe.rs`)

### Warum azyklische Konfiguration?

Vor dem Eintritt in den Op-Modus muss die Klemme über **SDO-Writes** parametriert
werden (Motorstrom, Regelparameter, Betriebsmodus, …). Das geschieht genau einmal
im Übergang PreOp → SafeOp und ist unabhängig vom zyklischen Datenaustausch.

### Aufbau von `EL7037Configuration`

```rust
pub struct EL7037Configuration {
    pub encoder:          EncConfiguration,          // CoE 0x8000
    pub stm_motor:        StmMotorConfiguration,     // CoE 0x8010
    pub stm_controller_1: StmControllerConfiguration,// CoE 0x8011 (nur kp/ki bei EL7037)
    pub stm_features:     StmFeatures,               // CoE 0x8012
    pub pos_configuration: PosConfiguration,         // CoE 0x9020
    pub pos_features:     PosFeatures,               // CoE 0x9020/...
    pub pdo_assignment:   EL7037PredefinedPdoAssignment,
}
```

`Default::default()` setzt alle Felder auf die im Datenblatt definierten
Standardwerte. Besonderheit: `stm_features` setzt `select_info_data_1 =
MotorLoad` und `select_info_data_2 = MotorDcCurrent` (EL7037-spezifisch).

### CoE-Indizes im Überblick

| Objekt | CoE-Index | Beschreibung |
|---|---|---|
| `EncConfiguration` | 0x8000 | Encoder: Zähler-Modus, Filter, … |
| `StmMotorConfiguration` | 0x8010 | Motor: Maximalstrom, Spannung, Wicklungswiderstand, … |
| `StmControllerConfiguration` | 0x8011 | Regler: kp (0x8011:01), ki (0x8011:02) |
| `StmFeatures` | 0x8012 | Betriebsmodus (0x8012:01), Speed-Range (0x8012:05), Info-Daten |
| `PosConfiguration` | 0x9020 | Positioniereinheit: Zielgeschwindigkeit, Rampe, … |
| `PosFeatures` | 0x9020/… | Positionierfeatures: Emergency-Rampe, … |
| PDO-Assignments | 0x1C12 / 0x1C13 | RxPDO / TxPDO Zuordnung |

### Traits `Configuration` und `ConfigurableDevice`

```rust
// coe.rs
pub trait Configuration {
    fn write_config(&self, channel: EtherCATThreadChannel, device_address: u16)
        -> Result<(), anyhow::Error>;
}

pub trait ConfigurableDevice<C: Configuration + Clone> {
    fn write_config(&mut self, channel, device_address, config: &C)
        -> Result<(), anyhow::Error>;
    fn get_config(&self) -> C;
}
```

`EL7037Configuration` implementiert `Configuration` indem es alle Sub-Configs
der Reihe nach ausführt und zum Schluss die PDO-Assignments schreibt.
`ConfigurableDevice<EL7037Configuration> for EL7037` delegiert an die Config und
aktualisiert danach `self.configuration`, `self.txpdo`, `self.rxpdo`.

---

## 6. IO-Abstraktion: `StepperVelocityEL70x1Device`

### Warum eine gemeinsame Abstraktion?

EL7031, EL7037 und EL7041 steuern alle Schrittmotoren im Velocity-Modus.
Obwohl ihre CoE-Objekte leicht differieren, ist die semantische Schnittstelle
identisch: Sollgeschwindigkeit setzen, Position lesen, aktivieren/deaktivieren.
Das Trait `StepperVelocityEL70x1Device` (in
`io/stepper_velocity_el70x1.rs`) abstrahiert genau das.

### Datenobjekte

```rust
pub struct StepperVelocityEL70x1Input {
    pub counter_value: i128,       // absoluter Encoder-Zählwert (aufgelaufen)
    pub ready_to_enable: bool,
    pub ready: bool,
    pub warning: bool,
    pub error: bool,
    pub moving_positive: bool,
    pub moving_negative: bool,
    pub torque_reduced: bool,
}

pub struct StepperVelocityEL70x1Output {
    pub velocity: i16,             // roher PDO-Velocity-Wert
    pub enable: bool,
    pub reduce_torque: bool,
    pub reset: bool,
    pub set_counter: Option<i128>, // optionaler Counter-Override
}
```

### Implementierung in der EL7037

`mod.rs` implementiert das Trait:
- `set_output` / `get_input` / `get_output` prüfen zunächst, ob
  `operation_mode == DirectVelocity` gesetzt ist (panic / Err sonst).
- `get_speed_range` liefert `self.configuration.stm_features.speed_range`.
- Default-Methoden wie `set_speed(steps_per_second: f64)`, `get_speed()`,
  `set_position(i128)`, `get_digital_input(port)` müssen **nicht** überschrieben
  werden.

---

## 7. Gerätelebenszyklus & Verarbeitungs-Hooks

### `NewEthercatDevice`

```rust
impl NewEthercatDevice for EL7037 {
    fn new() -> Self {
        let configuration = EL7037Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
            counter_wrapper: CounterWrapperU16U128::new(),
        }
    }
}
```

Der parameterlose Konstruktor wird von `device_from_subdevice_identity` in
`devices/mod.rs` aufgerufen, sobald eine EL7037 auf dem Bus erkannt wird.

### `EthercatDeviceProcessing`

Pro EtherCAT-Zyklus werden zwei Hooks aufgerufen:

```
EtherCAT-Frame empfangen
        ↓
input_post_process()   ← TxPDO-Daten wurden gelesen; Counter-Wrapper aktualisieren
        ↓
Applikations-Logik (set_output, …)
        ↓
output_pre_process()   ← RxPDO-Daten werden gleich gesendet; Fehler-Reset & Counter-Set
        ↓
EtherCAT-Frame senden
```

**`input_post_process`** aktualisiert `counter_wrapper` mit dem aktuellen
`enc_status_compact.counter_value` sowie den Overflow-/Underflow-Flags.

**`output_pre_process`** erledigt drei Dinge:
1. **Fehler-Reset**: Ist `stm_status.error == true`, wird `stm_control.reset =
   true` gesetzt.
2. **Overflow-/Underflow-Quittierung**: Treten Counter-Grenzen auf, wird der
   aktuelle Zählerwert per `enc_control_compact.set_counter = true` zurückgeschrieben,
   damit die Klemme die Flags löscht.
3. **Counter-Override-Konsumption**: Falls ein `push_override` aussteht (z. B.
   durch `set_position`), wird der neue Zielwert als `set_counter_value`
   eingetragen und das Override-Queue geleert.

---

## 8. Helper

### `CounterWrapperU16U128`

**Problem:** Der Encoder-PDO-Wert (`enc_status_compact.counter_value`) ist ein
`u16` und läuft nach 65535 wieder auf 0 — oder bei Rückwärtsbewegung von 0 auf
65535. Die Applikation benötigt aber einen absoluten `i128`-Zähler.

**Lösung:** Der `CounterWrapperU16U128` erkennt **steigende Flanken** der
`counter_overflow`- und `counter_underflow`-Flags der Klemme und addiert/
subtrahiert entsprechend `u16::MAX + 1` auf den akkumulierten Zählwert.

```rust
counter_wrapper.update(counter_value, underflow, overflow);
// liefert danach:
counter_wrapper.current() // -> i128
```

Für das **Setzen einer absoluten Position** gibt es eine asynchrone Queue:

```rust
counter_wrapper.push_override(new_position_i128);
// Im nächsten output_pre_process wird pop_override() abgefragt und
// in set_counter / set_counter_value übersetzt.
```

### `EL70x1VelocityConverter`

**Problem:** Das `stm_velocity`-PDO ist ein `i16` mit der Konvention
`±32767 (i16::MAX) = ±100 %` der konfigurierten `EL70x1SpeedRange`.
Die Applikation denkt aber in **Schritten pro Sekunde**.

**Lösung:** `EL70x1VelocityConverter::new(&speed_range)` leitet daraus
`max_steps_per_second` ab (z. B. `EL70x1SpeedRange::Steps4000 → 4000`).

```rust
let conv = EL70x1VelocityConverter::new(&self.configuration.stm_features.speed_range);
let vel: i16 = conv.steps_to_velocity(steps_per_second, /*probabilistic_rounding=*/ true);
let steps: i32 = conv.velocity_to_steps(vel, true);
```

Der `probabilistic_rounding`-Parameter aktiviert **stochastisches Runden**: statt
bei Bruchteilen immer abzuschneiden, wird der Bruchteil als Wahrscheinlichkeit
für ein Aufrunden verwendet (`rand::random_bool`). Das vermeidet systematische
Geschwindigkeitsfehler bei kleinen Sollwerten nahe der Auflösungsgrenze.

---

## 9. Schritt-für-Schritt: Einen neuen EL70xx-Treiber implementieren

Die EL7037 dient hier als Vorlage. Das Vorgehen für eine neue Klemme
(z. B. eine EL7039) ist immer gleich:

### Schritt 1 — Identität ermitteln

Die drei Werte (Vendor ID, Product Code, Revision) stehen im EEPROM der Klemme.
Auslesen mit dem `ethercat-eeprom-dump`-Skill:

```
Vendor ID:  0x00000002
Product Code: 0x1b7d3052   ← EL7037-Beispiel
Revision:   0x00170000
```

### Schritt 2 — `pdo.rs` anlegen

1. `TxPdo`-Struct mit `Option<...>`-Feldern und `#[pdo_object_index(0x1Axx)]`.
2. `RxPdo`-Struct analog (`#[pdo_object_index(0x16xx)]`).
3. `PredefinedPdoAssignment`-Enum — einen Zweig pro Beckhoff-definierter
   PDO-Kombination; `Default` auf den häufigsten Modus setzen.
4. `impl PredefinedPdoAssignment<TxPdo, RxPdo>` — in `txpdo_assignment` /
   `rxpdo_assignment` festlegen, welche Felder `Some` sind.
5. Unit-Tests mit den erwarteten Byte-Größen aus dem Datenblatt hinzufügen.

### Schritt 3 — `coe.rs` anlegen

1. `*Configuration`-Struct aggregiert die passenden Sub-Configs aus
   `shared_config::el70x7` (oder `el70x1`, je nach Klemmentyp).
2. `impl Default` → Datenblatt-Defaults.
3. `impl Configuration` → ruft alle `sub.write_config(...)` der Reihe nach auf.
4. `impl ConfigurableDevice<*Configuration> for EL70xx` → delegiert + speichert
   Config + aktualisiert tx/rxpdo.

### Schritt 4 — `mod.rs` anlegen

1. `#[derive(Debug, Clone, EthercatDevice)]`-Struct mit Feldern:
   `txpdo`, `rxpdo`, `is_used: bool`, `configuration`, `counter_wrapper`.
2. `impl NewEthercatDevice` → `new()` baut Default-Config + initiale PDOs.
3. `impl EthercatDeviceProcessing` → `input_post_process` und
   `output_pre_process` implementieren.
4. `impl StepperVelocityEL70x1Device` → `set_output`, `get_input`, `get_output`,
   `get_speed_range`, `get_port_count` (und `get_digital_in_port_count` etc.).
5. Identitätskonstanten am Ende der Datei.

### Schritt 5 — In `devices/mod.rs` registrieren

```rust
pub mod el70xx_neu;
use el70xx_neu::EL70XX_NEU_IDENTITY_A;

// In device_from_subdevice_identity (Box-Variante):
EL70XX_NEU_IDENTITY_A => Ok(Box::new(el70xx_neu::EL70xxNeu::new())),

// In der zweiten (Rc<RefCell>-)Variante ebenso.
```

### Referenz-Implementierungen

- **EL7037** (dieses Verzeichnis) — mit `shared_config::el70x7`, bester
  Ausgangspunkt für neue EL70x7-Varianten.
- **EL7031** (`devices/el7031/`) — nutzt `shared_config::el70x1`, bester
  Ausgangspunkt für EL70x1-kompatible Klemmen.

Weiterführende Skills im `control`-Repo:
- `.claude/skills/ethercat-hal/SKILL.md` — Crate-Überblick und Konventionen
- `.claude/skills/creating-a-machine/SKILL.md` — Integration einer Klemme in
  eine Maschine
- `.claude/skills/ethercat-eeprom-dump/SKILL.md` — Identitätswerte auslesen

---

## Zusammenfassung der beteiligten Dateien

| Datei | Zweck |
|---|---|
| `devices/el7037/mod.rs` | Gerätestruct, alle Trait-Impls, Identitätskonstanten |
| `devices/el7037/coe.rs` | Azyklische Konfiguration, CoE-Objekthierarchie |
| `devices/el7037/pdo.rs` | Zyklische PDO-Structs, Betriebsmodus-Enum, Tests |
| `devices/mod.rs` | Globale Registrierung (Match auf Identitätstupel) |
| `ethercat_hal_derive/src/lib.rs` | Derive-Makros `EthercatDevice`, `TxPdo`, `RxPdo`, `PdoObject` |
| `shared_config/el70x7.rs` | EL7037-spezifische Sub-Configs (0x8010–0x8012) |
| `pdo/el70x7.rs` | PDO-Objekt-Implementierungen (Bit-Mapping via bitvec) |
| `io/stepper_velocity_el70x1.rs` | Gemeinsames Stepper-IO-Trait für EL703x/EL704x |
| `helpers/counter_wrapper_u16_i128.rs` | u16-Encoder-Counter → i128 akkumuliert |
| `helpers/el70xx_velocity_converter.rs` | Schritte/s ↔ i16-PDO-Velocity |
