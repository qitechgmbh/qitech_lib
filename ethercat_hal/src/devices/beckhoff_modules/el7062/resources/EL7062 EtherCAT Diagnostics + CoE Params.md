## 7.5 Diagnostics

### 7.5.1 Diagnostics - basic principles of diag messages

DiagMessages designates a system for the transmission of messages from the EtherCAT Slave to the EtherCAT Master/TwinCAT. The messages are stored by the device in its own CoE under 0x10F3 and can be read by the application or the System Manager. An error message referenced via a code is output for each event stored in the device (warning, error, status change).

**Definition**

The DiagMessages system is defined in the ETG (EtherCAT Technology Group) in the guideline ETG.1020, chapter 13 “Diagnosis handling”. It is used so that pre-defined or flexible diagnostic messages can be conveyed from the EtherCAT Slave to the Master. In accordance with the ETG, the process can therefore be implemented supplier-independently. Support is optional. The firmware can store up to 250 DiagMessages in its own CoE.

Each DiagMessage consists of:
- Diag Code (4-byte)
- Flags (2-byte; info, warning or error)
- Text ID (2-byte; reference to explanatory text from the ESI/XML)
- Timestamp (8-byte, local slave time or 64-bit Distributed Clock time, if available)
- Dynamic parameters added by the firmware

The DiagMessages are explained in text form in the ESI/XML file belonging to the EtherCAT device: on the basis of the Text ID contained in the DiagMessage, the corresponding plain text message can be found in the languages contained in the ESI/XML. In the case of Beckhoff products these are usually German and English.

Via the entry NewMessagesAvailable the user receives information that new messages are available. DiagMessages can be confirmed in the device: the last/latest unconfirmed message can be confirmed by the user.

In the CoE both the control entries and the history itself can be found in the CoE object 0x10F3:

*Fig. 8: DiagMessages in the CoE*

The subindex of the latest DiagMessage can be read under 0x10F3:02.

**Support for commissioning**

The DiagMessages system is to be used above all during the commissioning of the plant. The diagnostic values e.g. in the StatusWord of the device (if available) are helpful for online diagnosis during the subsequent continuous operation.

**TwinCAT System Manager implementation**

From TwinCAT 2.11 DiagMessages, if available, are displayed in the device’s own interface. Operation (collection, confirmation) also takes place via this interface.

*Fig. 9: Implementation of the DiagMessage system in the TwinCAT System Manager*

The operating buttons (B) and the history read out (C) can be seen on the Diag History tab (A). The components of the message:
- Info/Warning/Error
- Acknowledge flag (N = unconfirmed, Q = confirmed)
- Time stamp
- Text ID
- Plain text message according to ESI/XML data

The meanings of the buttons are self-explanatory.

**DiagMessages within the ADS Logger/Eventlogger**

From TwinCAT 3.1 build 4022 onwards, DiagMessages sent by the terminal are shown by the TwinCAT ADS Logger. Given that DiagMessages are represented IO-comprehensive at one place, commissioning will be simplified. In addition, the logger output could be stored into a data file – hence DiagMessages are available long-term for analysis.

DiagMessages are actually only available locally in CoE 0x10F3 in the terminal and can be read out manually if required, e.g. via the DiagHistory mentioned above.

In the latest developments, the EtherCAT Terminals are set by default to report the presence of a DiagMessage as emergency via EtherCAT; the event logger can then retrieve the DiagMessage. The function is activated in the terminal via 0x10F3:05, so such terminals have the following entry in the StartUp list by default:

*Fig. 10: Startup List*

If the function is to be deactivated because, for example, many messages come in or the EventLogger is not used, the StartUp entry can be deleted or set to 0. The value can then be set back to 1 later from the PLC via CoE access if required.

**Reading messages into the PLC**

- In preparation -

**Interpretation**

**Time stamp**

The time stamp is obtained from the local clock of the terminal at the time of the event. The time is usually the distributed clock time (DC) from register x910.

Please note: When EtherCAT is started, the DC time in the reference clock is set to the same time as the local IPC/TwinCAT time. From this moment the DC time may differ from the IPC time, since the IPC time is not adjusted. Significant time differences may develop after several weeks of operation without a EtherCAT restart. As a remedy, external synchronization of the DC time can be used, or a manual correction calculation can be applied, as required: The current DC time can be determined via the EtherCAT master or from register x901 of the DC slave.

**Structure of the Text ID**

The structure of the MessageID is not subject to any standardization and can be supplier-specifically defined. In the case of Beckhoff EtherCAT devices (EL, EP) it usually reads according to xyzz:

| x | y | zz |
| :--- | :--- | :--- |
| 0: Systeminfo<br>1: Info<br>2: reserved<br>4: Warning<br>8: Error | 0: System<br>1: General<br>2: Communication<br>3: Encoder<br>4: Drive<br>5: Inputs<br>6: I/O general<br>7: reserved | Error number |

Example: Message 0x4413 --> Drive Warning Number 0x13

**Overview of text IDs**

Specific text IDs are listed in the device documentation.

| Text ID | Type | Place | Text Message | Additional comment |
| :--- | :--- | :--- | :--- | :--- |
| 0x0001 | Information | System | No error | No error |
| 0x0002 | Information | System | Communication established | Connection established |
| 0x0003 | Information | System | Initialization: 0x%X, 0x%X, 0x%X | General information; parameters depend on event. See device documentation for interpretation. |
| 0x1000 | Information | System | Information: 0x%X, 0x%X, 0x%X | General information; parameters depend on event. See device documentation for interpretation. |
| 0x1012 | Information | System | EtherCAT state change Init - PreOp | - |
| 0x1021 | Information | System | EtherCAT state change PreOp - Init | - |
| 0x1024 | Information | System | EtherCAT state change PreOp - Safe-Op | - |
| 0x1042 | Information | System | EtherCAT state change SafeOp - PreOp | - |
| 0x1048 | Information | System | EtherCAT state change SafeOp - Op | - |
| 0x1084 | Information | System | EtherCAT state change Op - SafeOp | - |
| 0x1100 | Information | General | Detection of operation mode completed: 0x%X, %d | Detection of the mode of operation ended |
| 0x1135 | Information | General | Cycle time o.k.: %d | Cycle time OK |
| 0x1157 | Information | General | Data manually saved (Idx: 0x%X, SubIdx: 0x%X) | Data saved manually |
| 0x1158 | Information | General | Data automatically saved (Idx: 0x%X, SubIdx: 0x%X) | Data saved automatically |
| 0x1159 | Information | General | Data deleted (Idx: 0x%X, SubIdx: 0x%X) | Data deleted |
| 0x117F | Information | General | Information: 0x%X, 0x%X, 0x%X | Information |
| 0x1201 | Information | Communication | Communication re-established | Communication to the field side restored.<br>This message appears, for example, if the voltage was removed from the power contacts and re-applied during operation. |
| 0x1300 | Information | Encoder | Position set: %d, %d | Position set - StartInputhandler |
| 0x1303 | Information | Encoder | Encoder Supply ok | Encoder power supply unit OK |
| 0x1304 | Information | Encoder | Encoder initialization successfully, channel: %X | Encoder initialization successfully completed |
| 0x1305 | Information | Encoder | Sent command encoder reset, channel: %X | Send encoder reset command |
| 0x1400 | Information | Drive | Drive is calibrated: %d, %d | Drive is calibrated |
| 0x1401 | Information | Drive | Actual drive state: 0x%X, %d | Current drive status |
| 0x1705 | Information | | CPU usage returns in normal range (< 85%%) | Processor load is back in the normal range |
| 0x1706 | Information | | Channel is not in saturation anymore | Channel is no longer in saturation |
| 0x1707 | Information | | Channel is not in overload anymore | Channel is no longer overloaded |
| 0x170A | Information | | No channel range error anymore | A measuring range error is no longer active |
| 0x170C | Information | | Calibration data saved | Calibration data were saved |
| 0x170D | Information | | Calibration data will be applied and saved after sending the command “0x5AFE” | Calibration data are not applied and saved until the command "0x5AFE" is sent. |
| 0x2000 | Information | System | %s: %s | |
| 0x2001 | Information | System | %s: Network link lost | Network connection lost |
| 0x2002 | Information | System | %s: Network link detected | Network connection found |
| 0x2003 | Information | System | %s: no valid IP Configuration - Dhcp client started | Invalid IP configuration |
| 0x2004 | Information | System | %s: valid IP Configuration (IP: %d.%d.%d.%d) assigned by Dhcp server %d.%d.%d.%d | Valid IP configuration, assigned by the DHCP server |
| 0x2005 | Information | System | %s: Dhcp client timed out | DHCP client timeout |
| 0x2006 | Information | System | %s: Duplicate IP Address detected (%d.%d.%d.%d) | Duplicate IP address found |
| 0x2007 | Information | System | %s: UDP handler initialized | UDP handler initialized |
| 0x2008 | Information | System | %s: TCP handler initialized | TCP handler initialized |
| 0x2009 | Information | System | %s: No more free TCP sockets available | No free TCP sockets available. |
| 0x4000 | Warning | | Warning: 0x%X, 0x%X, 0x%X | General warning; parameters depend on event. See device documentation for interpretation. |
| 0x4001 | Warning | System | Warning: 0x%X, 0x%X, 0x%X | |
| 0x4002 | Warning | System | %s: %s Connection Open (IN:%d OUT:%d API:%dms) from %d.%d.%d.%d successful | |
| 0x4003 | Warning | System | %s: %s Connection Close (IN:%d OUT:%d) from %d.%d.%d.%d successful | |
| 0x4004 | Warning | System | %s: %s Connection (IN:%d OUT:%d) with %d.%d.%d.%d timed out | |
| 0x4005 | Warning | System | %s: %s Connection Open (IN:%d OUT:%d) from %d.%d.%d.%d denied (Error: %u) | |
| 0x4006 | Warning | System | %s: %s Connection Open (IN:%d OUT:%d) from %d.%d.%d.%d denied (Input Data Size expected: %d Byte(s) received: %d Byte(s)) | |
| 0x4007 | Warning | System | %s: %s Connection Open (IN:%d OUT:%d) from %d.%d.%d.%d denied (Output Data Size expected: %d Byte(s) received: %d Byte(s)) | |
| 0x4008 | Warning | System | %s: %s Connection Open (IN:%d OUT:%d) from %d.%d.%d.%d denied (RPI:%dms not supported -> API:%dms) | |
| 0x4101 | Warning | General | Terminal-Overtemperature | Overtemperature. The internal temperature of the terminal exceeds the parameterized warning threshold. |
| 0x4102 | Warning | General | Discrepancy in the PDO-Configuration | The selected PDOs do not match the set operating mode.<br>Sample: Drive operates in velocity mode, but the velocity PDO is but not mapped in the PDOs. |
| 0x417F | Warning | General | Warning: 0x%X, 0x%X, 0x%X | |
| 0x428D | Warning | General | Challenge is not Random | |
| 0x4300 | Warning | Encoder | Subincrements deactivated: %d, %d | Sub-increments deactivated (despite activated configuration) |
| 0x4301 | Warning | Encoder | Encoder-Warning | General encoder error |
| 0x4302 | Warning | Encoder | Maximum frequency of the input signal is nearly reached (channel %d) | |
| 0x4303 | Warning | Encoder | Limit counter value was reduced because of the PDO configuration (channel %d) | |
| 0x4304 | Warning | Encoder | Reset counter value was reduced because of the PDO configuration (channel %d) | |
| 0x4400 | Warning | Drive | Drive is not calibrated: %d, %d | Drive is not calibrated |
| 0x4401 | Warning | Drive | Starttype not supported: 0x%X, %d | Start type is not supported |
| 0x4402 | Warning | Drive | Command rejected: %d, %d | Command rejected |
| 0x4405 | Warning | Drive | Invalid modulo subtype: %d, %d | Modulo sub-type invalid |
| 0x4410 | Warning | Drive | Target overrun: %d, %d | Target position exceeded |
| 0x4411 | Warning | Drive | DC-Link undervoltage (Warning) | The DC link voltage of the terminal is lower than the parameterized minimum voltage. Activation of the output stage is prevented. |
| 0x4412 | Warning | Drive | DC-Link overvoltage (Warning) | The DC link voltage of the terminal is higher than the parameterized maximum voltage. Activation of the output stage is prevented. |
| 0x4413 | Warning | Drive | I2T-Model Amplifier overload (Warning) | • The amplifier is being operated outside the specification.<br>• The I2T-model of the amplifier is incorrectly parameterized. |
| 0x4414 | Warning | Drive | I2T-Model Motor overload (Warning) | • The motor is being operated outside the parameterized rated values.<br>• The I2T-model of the motor is incorrectly parameterized. |
| 0x4415 | Warning | Drive | Speed limitation active | The maximum speed is limited by the parameterized objects (e.g. velocity limitation, motor speed limitation). This warning is output if the set velocity is higher than one of the parameterized limits. |
| 0x4416 | Warning | Drive | Step lost detected at position: 0x%X%X | Step loss detected |
| 0x4417 | Warning | Drive | Motor overtemperature | The internal temperature of the motor exceeds the parameterized warning threshold |
| 0x4418 | Warning | Drive | Limit: Current | Limit: current is limited |
| 0x4419 | Warning | Drive | Limit: Amplifier I2T-model exceeds 100%% | The threshold values for the maximum current were exceeded. |
| 0x441A | Warning | Drive | Limit: Motor I2T-model exceeds 100%% | Limit: Motor I2T-model exceeds 100% |
| 0x441B | Warning | Drive | Limit: Velocity limitation | The threshold values for the maximum speed were exceeded. |
| 0x441C | Warning | Drive | STO while the axis was enabled | An attempt was made to activate the axis, despite the fact that no voltage is present at the STO input. |
| 0x4600 | Warning | General IO | Wrong supply voltage range | Supply voltage not in the correct range |
| 0x4610 | Warning | General IO | Wrong output voltage range | Output voltage not in the correct range |
| 0x4705 | Warning | | Processor usage at %d %% | Processor load at %d %% |
| 0x470A | Warning | | EtherCAT Frame missed (change Settings or DC Operation Mode or Sync0 Shift Time) | EtherCAT frame missed (change DC Operation Mode or Sync0 Shift Time under Settings) |
| 0x8000 | Error | System | %s: %s | |
| 0x8001 | Error | System | Error: 0x%X, 0x%X, 0x%X | General error; parameters depend on event. See device documentation for interpretation. |
| 0x8002 | Error | System | Communication aborted | Communication aborted |
| 0x8003 | Error | System | Configuration error: 0x%X, 0x%X, 0x%X | General; parameters depend on event. See device documentation for interpretation. |
| 0x8004 | Error | System | %s: Unsuccessful FwdOpen-Response received from %d.%d.%d.%d (%s) (Error: %u) | |
| 0x8005 | Error | System | %s: FwdClose-Request sent to %d.%d.%d.%d (%s) | |
| 0x8006 | Error | System | %s: Unsuccessful FwdClose-Response received from %d.%d.%d.%d (%s) (Error: %u) | |
| 0x8007 | Error | System | %s: Connection with %d.%d.%d.%d (%s) closed | |
| 0x8100 | Error | General | Status word set: 0x%X, %d | Error bit set in the status word |
| 0x8101 | Error | General | Operation mode incompatible to PDO interface: 0x%X, %d | Mode of operation incompatible with the PDO interface |
| 0x8102 | Error | General | Invalid combination of Inputs and Outputs PDOs | Invalid combination of input and output PDOs |
| 0x8103 | Error | General | No variable linkage | No variables linked |
| 0x8104 | Error | General | Terminal-Overtemperature | The internal temperature of the terminal exceeds the parameterized error threshold. Activation of the terminal is prevented |
| 0x8105 | Error | General | PD-Watchdog | Communication between the fieldbus and the output stage is secured by a Watchdog. The axis is stopped automatically if the fieldbus communication is interrupted.<br>• The EtherCAT connection was interrupted during operation.<br>• The Master was switched to Config mode during operation. |
| 0x8135 | Error | General | Cycle time has to be a multiple of 125 µs | The IO or NC cycle time divided by 125 µs does not produce a whole number. |
| 0x8136 | Error | General | Configuration error: invalid sampling rate | Configuration error: Invalid sampling rate |
| 0x8137 | Error | General | Electronic type plate: CRC error | Content of the external name plate memory invalid. |
| 0x8140 | Error | General | Sync Error | Real-time violation |
| 0x8141 | Error | General | Sync%X Interrupt lost | Sync%X Interrupt lost |
| 0x8142 | Error | General | Sync Interrupt asynchronous | Sync Interrupt asynchronous |
| 0x8143 | Error | General | Jitter too big | Jitter limit violation |
| 0x817F | Error | General | Error: 0x%X, 0x%X, 0x%X | |
| 0x8200 | Error | Communication | Write access error: %d, %d | Error while writing |
| 0x8201 | Error | Communication | No communication to field-side (Auxiliary voltage missing) | • There is no voltage applied to the power contacts.<br>• A firmware update has failed. |
| 0x8281 | Error | Communication | Ownership failed: %X | |
| 0x8282 | Error | Communication | To many Keys founded | |
| 0x8283 | Error | Communication | Key Creation failed: %X | |
| 0x8284 | Error | Communication | Key loading failed | |
| 0x8285 | Error | Communication | Reading Public Key failed: %X | |
| 0x8286 | Error | Communication | Reading Public EK failed: %X | |
| 0x8287 | Error | Communication | Reading PCR Value failed: %X | |
| 0x8288 | Error | Communication | Reading Certificate EK failed: %X | |
| 0x8289 | Error | Communication | Challenge could not be hashed: %X | |
| 0x828A | Error | Communication | Tickstamp Process failed | |
| 0x828B | Error | Communication | PCR Process failed: %X | |
| 0x828C | Error | Communication | Quote Process failed: %X | |
| 0x82FF | Error | Communication | Bootmode not activated | Boot mode not activated |
| 0x8300 | Error | Encoder | Set position error: 0x%X, %d | Error while setting the position |
| 0x8301 | Error | Encoder | Encoder increments not configured: 0x%X, %d | Encoder increments not configured |
| 0x8302 | Error | Encoder | Encoder error | The amplitude of the resolver is too small |
| 0x8303 | Error | Encoder | Encoder power missing (channel %d) | |
| 0x8304 | Error | Encoder | Encoder communication error, channel: %X | Encoder communication error |
| 0x8305 | Error | Encoder | EnDat2.2 is not supported, channel: %X | EnDat2.2 is not supported |
| 0x8306 | Error | Encoder | Delay time, tolerance limit exceeded, 0x%X, channel: %X | Runtime measurement, tolerance exceeded |
| 0x8307 | Error | Encoder | Delay time, maximum value exceeded, 0x%X, channel: %X | Runtime measurement, maximum value exceeded |
| 0x8308 | Error | Encoder | Unsupported ordering designation, 0x%X, channel: %X (only 02 and 22 is supported) | Wrong EnDat order ID |
| 0x8309 | Error | Encoder | Encoder CRC error, channel: %X | Encoder CRC error |
| 0x830A | Error | Encoder | Temperature %X could not be read, channel: %X | Temperature cannot be read |
| 0x830C | Error | Encoder | Encoder Single-Cycle-Data Error, channel. %X | CRC error detected. Check the transmission path and the CRC polynomial |
| 0x830D | Error | Encoder | Encoder Watchdog Error, channel. %X | The sensor has not responded within a predefined time period |
| 0x8310 | Error | Encoder | Initialisation error | |
| 0x8311 | Error | Encoder | Maximum frequency of the input signal is exceeded (channel %d) | |
| 0x8312 | Error | Encoder | Encoder plausibility error (channel %d) | |
| 0x8313 | Error | Encoder | Configuration error (channel %d) | |
| 0x8314 | Error | Encoder | Synchronisation error | |
| 0x8315 | Error | Encoder | Error status input (channel %d) | |
| 0x8400 | Error | Drive | Incorrect drive configuration: 0x%X, %d | Drive incorrectly configured |
| 0x8401 | Error | Drive | Limiting of calibration velocity: %d, %d | Limitation of the calibration velocity |
| 0x8402 | Error | Drive | Emergency stop activated: 0x%X, %d | Emergency stop activated |
| 0x8403 | Error | Drive | ADC Error | Error during current measurement in the ADC |
| 0x8404 | Error | Drive | Overcurrent | Overcurrent in phase U, V or W |
| 0x8405 | Error | Drive | Invalid modulo position: %d | Modulo position invalid |
| 0x8406 | Error | Drive | DC-Link undervoltage (Error) | The DC link voltage of the terminal is lower than the parameterized minimum voltage. Activation of the output stage is prevented. |
| 0x8407 | Error | Drive | DC-Link overvoltage (Error) | The DC link voltage of the terminal is higher than the parameterized maximum voltage. Activation of the output stage is prevented. |
| 0x8408 | Error | Drive | I2T-Model Amplifier overload (Error) | • The amplifier is being operated outside the specification.<br>• The I2T-model of the amplifier is incorrectly parameterized. |
| 0x8409 | Error | Drive | I2T-Model motor overload (Error) | • The motor is being operated outside the parameterized rated values.<br>• The I2T-model of the motor is incorrectly parameterized. |
| 0x840A | Error | Drive | Overall current threshold exceeded | Total current exceeded |
| 0x8415 | Error | Drive | Invalid modulo factor: %d | Modulo factor invalid |
| 0x8416 | Error | Drive | Motor overtemperature | The internal temperature of the motor exceeds the parameterized error threshold. The motor stops immediately. Activation of the output stage is prevented. |
| 0x8417 | Error | Drive | Maximum rotating field velocity exceeded | Rotary field speed exceeds the value specified for dual use (EU 1382/2014). |
| 0x841C | Error | Drive | STO while the axis was enabled | An attempt was made to activate the axis, despite the fact that no voltage is present at the STO input. |
| 0x8550 | Error | Inputs | Zero crossing phase %X missing | Zero crossing phase %X missing |
| 0x8551 | Error | Inputs | Phase sequence Error | Wrong direction of rotation |
| 0x8552 | Error | Inputs | Overcurrent phase %X | Overcurrent phase %X |
| 0x8553 | Error | Inputs | Overcurrent neutral wire | Overcurrent neutral wire |
| 0x8581 | Error | Inputs | Wire broken Ch %D | Wire broken Ch %d |
| 0x8600 | Error | General IO | Wrong supply voltage range | Supply voltage not in the correct range |
| 0x8601 | Error | General IO | Supply voltage to low | Supply voltage too low |
| 0x8602 | Error | General IO | Supply voltage to high | Supply voltage too high |
| 0x8603 | Error | General IO | Over current of supply voltage | Overcurrent of supply voltage |
| 0x8610 | Error | General IO | Wrong output voltage range | Output voltage not in the correct range |
| 0x8611 | Error | General IO | Output voltage to low | Output voltage too low |
| 0x8612 | Error | General IO | Output voltage to high | Output voltage too high |
| 0x8613 | Error | General IO | Over current of output voltage | Overcurrent of output voltage |
| 0x8700 | Error | | Channel/Interface not calibrated | Channel/interface not synchronized |
| 0x8701 | Error | | Operating time was manipulated | Operating time was manipulated |
| 0x8702 | Error | | Oversampling setting is not possible | Oversampling setting not possible |
| 0x8703 | Error | | No slave controller found | No slave controller found |
| 0x8704 | Error | | Slave controller is not in Bootstrap | Slave controller is not in bootstrap |
| 0x8705 | Error | | Processor usage to high (>= 100%%) | Processor load too high (>= 100%%) |
| 0x8706 | Error | | Channel in saturation | Channel in saturation |
| 0x8707 | Error | | Channel overload | Channel overload |
| 0x8708 | Error | | Overloadtime was manipulated | Overload time was manipulated |
| 0x8709 | Error | | Saturationtime was manipulated | Saturation time was manipulated |
| 0x870A | Error | | Channel range error | Measuring range error for the channel |
| 0x870B | Error | | no ADC clock | No ADC clock available |
| 0xFFFF | Information | | Debug: 0x%X, 0x%X, 0x%X | Debug: 0x%X, 0x%X, 0x%X |

### 7.5.2 Notes on Diag Messages associated with Motor Terminals

**„Ack. Message“ Button**

The ‚Ack. Message’ button has no effect on the Drive State Machine of the Motor terminals, pressing the button does not make an axis reset.
The Drive State Machine has no influence on the error list, an axis reset also does not remove any entries from the error list, however, this can be done by pressing the ‚Ack. Message’ button.

## 8 Advanced device information

### 8.1 CoE parameters

#### 8.1.1 Objects for parameterization

##### Index 8000 FB Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8000:0 | FB Settings Ch.1 | | UINT8 | RO | 0x1C (28dec) |
| 8000:11 | Device type | | UINT32 | RW | 0x00000005 (5dec) |
| 8000:12 | Singleturn bits | Number of single-turn bits in the position display (process data, CoE). The sum of the single-turn bits and multi-turn bits must be 32.<br>Note: This parameter only influences the display and is independent of the physical resolution of the position sensor. | UINT8 | RW | 0x14 (20dec) |
| 8000:13 | Multiturn bits | Number of multi-turn bits in the position display (process data, CoE). The sum of the single-turn bits and multi-turn bits must be 32.<br>Note: This parameter only influences the display and is independent of the physical resolution of the position sensor. | UINT8 | RW | 0x0C (12dec) |
| 8000:14 | Observer bandwidth | Bandwidth of the speed observer [Hz] | UINT16 | RW | 0x00C8 (200dec) |
| 8000:15 | Observer feed-forward | Load ratio [%] between internal rotor inertia of the motor and the total inertia of the driven system.<br>Load ratio = internal moment of inertia / (internal moment of inertia + mass moment of inertia of the load).<br>Examples:<br>• 100% = load-free<br>• 50 % = moments of inertia of input and output are equal | UINT8 | RW | 0x64 (100dec) |
| 8000:17 | Position offset | The Position offset is subtracted from the raw position of the encoder.<br>It can only be written with the axis stopped. | UINT32 | RW | 0x00000000 (0dec) |
| 8000:18 | Secondary position offset | The Secondary Position Offset is subtracted from the "Secondary Position".<br>It can only be written with the axis stopped. | UINT32 | RW | 0x00000000 (0dec) |
| 8000:19 | Gear ratio motor shaft revolutions | These parameters are used to scale all positions and speeds from the motor side to the load side of a gear unit.<br>"Gear ratio motor shaft revolutions" describes the number of motor revolutions required to achieve the number of load revolutions configured in "Gear ratio driving shaft revolutions".<br>Example: For a reduction gear in which 5 motor revolutions result in 2 load revolutions, set the parameters as follows:<br>• Motor shaft revolutions = 5<br>• Driving shaft revolutions = 2 | UINT32 | RW | 0x00000001 (1dec) |
| 8000:1A | Gear ratio driving shaft revolutions | | UINT32 | RW | 0x00000001 (1dec) |
| 8000:1B | Min position range limit | Lowest value for the position display of setpoints and actual values. If the value falls below this, an underflow to the value "Max position range limit" occurs.<br>"Min position range limit" must always be lower than "Max position range limit". | UINT32 | RW | 0x00000000 (0dec) |
| 8000:1C | Max position range limit | Highest value for the position display of setpoints and actual values. If this is exceeded, an overflow to the value "Min position range limit" occurs.<br>"Max position range limit" must always be higher than "Min position range limit". | UINT32 | RW | 0xFFFFFFFF (4294967295dec) |

##### Index 8001 FB Touch probe Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8001:0 | FB Touch probe Settings Ch.1 | | UINT8 | RO | 0x16 (22dec) |
| 8001:11 | Touch probe 1 source | Selection of the input signal for Touch probe 1.<br>Permitted values:<br>• 1: Touch probe input 1<br>• 5: Hardware zero impulse | INT16 | RW | 0x0001 (1dec) |
| 8001:12 | Touch probe 2 source | Selection of the input signal for Touch probe 2.<br>Permitted values:<br>• 2: Touch probe input 2<br>• 5: Hardware zero impulse | INT16 | RW | 0x0002 (2dec) |
| 8001:15 | Touch probe 1 position source | Selection of the position held by Touch probe 1.<br>Permitted values:<br>• 0: FB Position<br>• 1: FB Secondary Position | INT16 | RW | 0x0000 (0dec) |
| 8001:16 | Touch probe 2 position source | Selection of the position held by Touch probe 2.<br>Permitted values:<br>• 0: FB Position<br>• 1: FB Secondary Position | INT16 | RW | 0x0000 (0dec) |

##### Index 8008 FB Settings ENC Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8008:0 | FB Settings ENC Ch.1 | | UINT8 | RO | 0x13 (19dec) |
| 8008:01 | Invert feedback direction | Changes the counting direction of the encoder.<br>This parameter can be used to adapt the encoder direction of rotation to the motor direction of rotation. | BOOLEAN | RW | 0x00 (0dec) |
| 8008:12 | Encoder type | permitted values:<br>• 0: disabled<br>• 1: RS422 differential<br>• 2: TTL single ended<br>• 6: TTL single ended - input filters disabled<br>• 7: open collector | UINT16 | RW | 0x0000 (0dec) |
| 8008:13 | Encoder Increments per Revolution | Resolution of the encoder after 4-fold evaluation. | UINT32 | RW | 0x00001000 (4096dec) |

##### Index 8010 DRV Amplifier Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8010:0 | DRV Amplifier Settings Ch.1 | | UINT8 | RO | 0x73 (115dec) |
| 8010:01 | Enable TxPDOToggle | Show TxPDO Toggle in the status word (bit 10). | BOOLEAN | RW | 0x00 (0dec) |
| 8010:02 | Enable input cycle counter | 1: enabled<br>The Input cycle counter is a two-bit counter that is incremented with each process data cycle up to a maximum value of 3, after which it starts again at 0.<br>The low bit is represented in bit 10 and the high bit in bit 14 of the status word. | BOOLEAN | RW | 0x00 (0dec) |
| 8010:04 | Repeat find commutation | Repeat the commutation angle determination. (Effective for all FOC operation modes). | BOOLEAN | RW | 0x01 (1dec) |
| 8010:12 | Current loop integral time | Integral component current controller [0.1 ms]. | UINT16 | RW | 0x0032 (50dec) |
| 8010:13 | Current loop proportional gain | Proportional component current controller [0.1 V/A] | UINT16 | RW | 0x0032 (50dec) |
| 8010:14 | Velocity loop integral time | Integral component velocity controller [0.1 ms]. | UINT32 | RW | 0x0000001E (30dec) |
| 8010:15 | Velocity loop proportional gain | Proportional component velocity controller [mA/(rad/s)]. | UINT32 | RW | 0x00000096 (150dec) |
| 8010:17 | Position loop proportional gain | Proportional component position controller [1/s] | UINT32 | RW | 0x0000000A (10dec) |
| 8010:31 | Velocity limitation | Limitation of the drive speed set value [1/min]. (Only effective in the CSV and CSP controller operation modes)<br>When using a gear ratio, this parameter refers to the load side. | UINT32 | RW | 0x000186A0 (100000dec) |
| 8010:32 | Short-Circuit Brake duration max | Max. duration of armature short circuit brake. [ms] | UINT16 | RW | 0x0000 (0dec) |
| 8010:33 | Stand still window | Tolerance window for standstill monitoring [1/min] | UINT16 | RW | 0x0001 (1dec) |
| 8010:39 | Select info data 1 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x02 (2dec) |
| 8010:3A | Select info data 2 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x04 (4dec) |
| 8010:49 | Halt ramp deceleration | Halt ramp deceleration [0.1 rad / s²] | UINT32 | RW | 0x0000F570 (62832dec) |
| 8010:50 | Following error window | Following error monitoring: following error window.<br>The value 0xFFFFFFFFFF (4294967295dec) disables the following error monitoring. | UINT32 | RW | 0xFFFFFFFF (4294967295dec) |
| 8010:51 | Following error time out | Following error monitoring: timeout [ms]. | UINT16 | RW | 0x0000 (0dec) |
| 8010:52 | Fault reaction option code | permitted values:<br>• 0: Disable drive function, motor is free to rotate<br>• 1: Slow down on slow down ramp<br>• 65534dec: Short circuit brake | UINT16 | RW | 0x0001 (1dec) |
| 8010:54 | Feature bits | | UINT32 | RW | 0x00000000 (0dec) |
| 8010:57 | Position loop velocity feed forward gain | Scaling factor for velocity pre-control from the position interpolator. | UINT8 | RW | 0x64 (100dec) |
| 8010:58 | Select info data 3 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x0A (10dec) |
| 8010:59 | Error suppression mask | | UINT32 | RW | 0x00000000 (0dec) |
| 8010:62 | Position loop deadband window | Deadband window of the position controller.<br>Unit: corresponds to the process data scaling of the set position and actual positions. | UINT32 | RW | 0x00000000 (0dec) |
| 8010:63 | Find commutation time | For field-oriented control (FOC) with an incremental encoder, commutation determination is required, during which nominal current is applied to the motor. The "Find commutation time" describes the time used for this. | UINT16 | RW | 0x000A (10dec) |
| 8010:64 | Commutation type | permitted values:<br>• 16: Stepper with internal counter<br>• 17: Stepper with encoder<br>• 18: Stepper FOC with encoder | UINT8 | RW | 0x10 (16dec) |
| 8010:65 | Invert direction of rotation | Inverting the direction of rotation.<br>This parameter inverts all setpoints and actual values and is used to ensure that the motor rotates in the correct direction for the application.<br>Note: This parameter is not suitable for adapting the directions of rotation of the encoder and motor to each other. Use 8008:01 "Invert feedback direction" for this purpose. | BOOLEAN | RW | 0x00 (0dec) |
| 8010:6D | Torque feed forward gain | Internal torque pre-control: scaling factor | UINT32 | RW | 0x00000064 (100dec) |
| 8010:6E | Torque feed forward filter time | Internal torque pre-control: filter time. [0.1 ms] | UINT32 | RW | 0x0000000A (10dec) |
| 8010:6F | Torque offset | Torque offset.<br>The value is given in thousandths of the nominal current. | INT16 | RW | 0x0000 (0dec) |
| 8010:70 | Torque limitation option code | Selection of the behavior in the CST controller operation mode (“Cyclic Synchronous Torque”).<br>Permitted values:<br>• 0: VeloLimitHasNoEffect<br>• 1: TorqueMightBeReducedToZero<br>• 2: TorqueMightBeReducedToRampPosNeg<br>• 3: TorqueMightBeReducedToRampPosMaxTorqueNeg<br>• 4: TorqueMightBeReducedToMaxTorquePosNeg | INT8 | RW | 0x00 (0dec) |
| 8010:72 | Stand still torque limitation | Only valid for commutation types "Stepper with internal counter" and "Stepper with encoder" (can be set via parameter 8010:64).<br>Configures a current reduction at standstill, i.e. when the target velocity is within the "Stand still window" (parameter 8010:33).<br>The value is given in thousandths of the nominal current. | UINT16 | RW | 0x7FFF (32767dec) |
| 8010:73 | Acceleration limitation | Limits the maximum acceleration or deceleration. [0.1 rad/s²] | UINT32 | RW | 0x0000F570 (62832dec) |

##### Index 8011 DRV Motor Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8011:0 | DRV Motor Settings Ch.1 | | UINT8 | RO | 0x34 (52dec) |
| 8011:12 | Rated current | The nominal current of the motor (data sheet value). Used for scaling "Torque actual value" and "Target torque". | UINT32 | RW | 0x00000BB8 (3000dec) |
| 8011:16 | Torque constant | Force constant of the motor. | UINT32 | RW | 0x0000012C (300dec) |
| 8011:18 | Rotor moment of inertia | Moment of inertia of the motor. | UINT32 | RW | 0x000001EF (495dec) |
| 8011:19 | Winding inductance | Winding inductance. | UINT16 | RW | 0x0186 (390dec) |
| 8011:1B | Motor speed limitation | Speed limit of the motor.<br>When using a gear ratio, this limit still refers to the motor side. | UINT32 | RW | 0x000186A0 (100000dec) |
| 8011:2E | Rated speed | Nominal speed of the motor. | UINT32 | RW | 0x000003E8 (1000dec) |
| 8011:30 | Winding resistance | Winding resistance of the motor. | UINT32 | RW | 0x00000578 (1400dec) |
| 8011:31 | Voltage constant | Voltage constant of the motor. | UINT32 | RW | 0x00004E20 (20000dec) |
| 8011:33 | Motor fullsteps per revolution | Number of full motor steps per revolution. | UINT32 | RW | 0x000000C8 (200dec) |
| 8011:34 | Configured motor current | Configured motor current. If this is smaller than the "Rated current", the motor current is limited to the smaller of the two values.<br>This value is used to distribute the load between the two channels of the terminal. | UINT32 | RW | 0x00000BB8 (3000dec) |

##### Index 8012 DRV Brake Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8012:0 | DRV Brake Settings Ch.1 | | UINT8 | RO | 0x14 (20dec) |
| 8012:01 | Enable manual override | • True: The brake state is forced manually via the CoE.<br>• False: The brake is automatically controlled by the drive controller. | BOOLEAN | RW | 0x00 (0dec) |
| 8012:02 | Manual brake state | Permitted values:<br>• 0: Release<br>• 1: Apply | BIT1 | RW | 0x00 (0dec) |
| 8012:05 | Brake option | Permitted values:<br>• 0: Enable output to release brake (for brakes that are released in the energized state)<br>• 1: Disable output to release brake (for brakes that are released in a de-energized state) | BIT4 | RW | 0x00 (0dec) |
| 8012:09 | External override | Enables the brake to be released via an external hardware signal.<br>The brake can only be released via the external signal, not applied.<br>You can select which hardware input (Touch probe 1 / Touch probe 2) is used for this. It is also possible to restrict whether this configuration is always active or only in the EtherCAT states INIT/PREOP/SAFEOP, i.e. when the drive is not fully operational (e.g. during maintenance work)<br>Permitted values:<br>• 0: Disabled<br>• 2: Digital Input 1<br>• 3: Digital Input 1 (only INIT/PREOP/SAFEOP)<br>• 4: Digital Input 2<br>• 5: Digital Input 2 (only INIT/PREOP/SAFEOP) | UINT8 | RW | 0x00 (0dec) |
| 8012:11 | Release delay | Time required for the holding brake to release after the current has been applied. | UINT16 | RW | 0x0000 (0dec) |
| 8012:12 | Application delay | Time required for the holding brake to apply after the current was switched off. | UINT16 | RW | 0x0000 (0dec) |
| 8012:13 | Emergency application timeout | Time that the amplifier waits for the speed to reach the standstill limit after a stop request.<br>If the waiting time is exceeded, the holding brake is triggered, regardless of the speed.<br>Note: This parameter must be set at least to the longest time the axis needs to come to a standstill after it has been switched torque-free.<br>For vertical axes, this parameter should be set to a low value to prevent the axis or load from falling very far.<br>Unit: ms | UINT16 | RW | 0x0000 (0dec) |
| 8012:14 | Brake moment of inertia | Moment of inertia of the brake.<br>Unit: g cm² | UINT16 | RW | 0x0000 (0dec) |

##### Index 8013 DRV Filter Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8013:0 | DRV Filter Settings Ch.1 | | UINT8 | RO | 0x19 (25dec) |
| 8013:10 | Low pass frequency 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:11 | Low pass damping 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:12 | High pass frequency 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:13 | High pass damping 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:14 | Filter type 1 | permitted values:<br>• 0: No_Filter<br>• 1: Low_pass_filter_1_order<br>• 2: Phase_correction_filter_1_order<br>• 3: Low_pass_filter_2_order<br>• 4: Phase_correction_filter_2_order<br>• 5: Notch_filter | INT16 | RW | 0x0000 (0dec) |
| 8013:15 | Low pass frequency 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:16 | Low pass damping 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:17 | High pass frequency 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:18 | High pass damping 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8013:19 | Filter type 2 | permitted values:<br>• 0: No_Filter<br>• 1: Low_pass_filter_1_order<br>• 2: Phase_correction_filter_1_order<br>• 3: Low_pass_filter_2_order<br>• 4: Phase_correction_filter_2_order<br>• 5: Notch_filter | INT16 | RW | 0x0000 (0dec) |

##### Index 801F DRV Vendor data Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 801F:0 | DRV Vendor data Ch.1 | | UINT8 | RO | 0x1C (28dec) |
| 801F:12 | Amplifier rated current | Maximum current per channel (without fan). | UINT32 | RW | 0x00001388 (5000dec) |
| 801F:14 | Amplifier overcurrent threshold | Switching threshold for the overcurrent switch-off. | UINT32 | RW | 0x00002710 (10000dec) |
| 801F:15 | Max rotary field frequency | | UINT16 | RW | 0x0257 (599dec) |
| 801F:17 | Amplifier rated current with fan | Maximum current per channel (with fan) | UINT32 | RW | 0x00001770 (6000dec) |
| 801F:18 | Vendor feature bits | | UINT32 | RW | 0x00000000 (0dec) |
| 801F:1A | Amplifier Rated Sum Current | Maximum sum current for all channels (without fan) | UINT32 | RW | 0x00001770 (6000dec) |
| 801F:1C | Amplifier Rated Sum Current with Fan | Maximum sum current for all channels (with fan) | UINT32 | RW | 0x00002710 (10000dec) |

##### Index 8060 DMC Settings Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8060:0 | DMC Settings Ch.1 | | UINT8 | RO | 0x17 (23dec) |
| 8060:07 | Emergency deceleration | Deceleration for the emergency stop ramp. (In ms from motor nominal speed to standstill)<br>Unit: 1 ms | UINT16 | RW | 0x0064 (100dec) |
| 8060:08 | Calibration position | If homing is successful, the "Actual position" is set to this value. | INT64 | RW | |
| 8060:09 | Calibration velocity (towards plc cam) | Velocity when hitting the cam in 10000ths of the motor nominal speed. | INT16 | RW | 0x0064 (100dec) |
| 8060:0A | Calibration Velocity (off plc cam) | Velocity when driving off the cam in 10000ths of the motor nominal speed. | INT16 | RW | 0x000A (10dec) |
| 8060:0E | Modulo factor | Feedback increments for one mechanical revolution. | INT64 | RW | |
| 8060:12 | Block calibration torque limit | Torque limitation for approaching the end stop. In parts per thousand of the nominal motor current. | UINT16 | RW | 0x0064 (100dec) |
| 8060:13 | Block calibration stop distance | After reaching the calibration position, the axis moves out of the end position by this distance. | INT64 | RW | |
| 8060:14 | Block calibration lag threshold | When this following error is exceeded, the axis is in the end position. | INT64 | RW | |
| 8060:15 | Target position window | Target position window:<br>The In-Target bit is set when the axis is within this window for at least the time set in 8060:16. | INT64 | RW | |
| 8060:16 | Target position monitor time | see 8060:15<br>Unit: ms | UINT16 | RW | 0x0014 (20dec) |
| 8060:17 | Target position timeout | When the setpoint generator has reached its end position and the axis is not in the target window after this time has elapsed, the task is terminated and the in-target bit is not set.<br>Unit: ms | UINT16 | RW | 0x1770 (6000dec) |

##### Index 8061 DMC Features Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8061:0 | DMC Features Ch.1 | | UINT8 | RO | 0x1B (27dec) |
| 8061:13 | Invert calibration cam search direction | Invert the direction of movement to search for the limit switch.<br>Default: FALSE = search with positive direction of rotation. | BOOLEAN | RW | 0x00 (0dec) |
| 8061:14 | Invert sync impulse search direction | Invert the direction of rotation to exit the limit switch.<br>Default: TRUE = exit in negative direction of rotation. | BOOLEAN | RW | 0x01 (1dec) |
| 8061:19 | Calibration cam source | Source for the reference switch.<br>• 0: Input 1<br>• 1: Input 2 | UINT8 | RW | 0x00 (0dec) |
| 8061:1A | Calibration cam active level | State of the reference switch in the actuated state.<br>• 0: Hi<br>• 1: Low | UINT8 | RW | 0x00 (0dec) |
| 8061:1B | Latch source | Source for the latch unit.<br>• 0: Input 1<br>• 1: Input 2 | UINT8 | RW | 0x00 (0dec) |

##### Index 8100 FB Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8100:0 | FB Settings Ch.2 | | UINT8 | RO | 0x1C (28dec) |
| 8100:11 | Device type | | UINT32 | RW | 0x00000005 (5dec) |
| 8100:12 | Singleturn bits | Number of single-turn bits in the position display (process data, CoE). The sum of the single-turn bits and multi-turn bits must be 32.<br>Note: This parameter only influences the display and is independent of the physical resolution of the position sensor. | UINT8 | RW | 0x14 (20dec) |
| 8100:13 | Multiturn bits | Number of multi-turn bits in the position display (process data, CoE). The sum of the single-turn bits and multi-turn bits must be 32.<br>Note: This parameter only influences the display and is independent of the physical resolution of the position sensor. | UINT8 | RW | 0x0C (12dec) |
| 8100:14 | Observer bandwidth | Bandwidth of the speed observer [Hz] | UINT16 | RW | 0x00C8 (200dec) |
| 8100:15 | Observer feed-forward | Load ratio [%] between internal rotor inertia of the motor and the total inertia of the driven system.<br>Load ratio = internal moment of inertia / (internal moment of inertia + mass moment of inertia of the load).<br>Examples:<br>• 100% = load-free<br>• 50 % = moments of inertia of input and output are equal | UINT8 | RW | 0x64 (100dec) |
| 8100:17 | Position offset | The Position offset is subtracted from the raw position of the encoder.<br>It can only be written with the axis stopped. | UINT32 | RW | 0x00000000 (0dec) |
| 8100:18 | Secondary position offset | The Secondary Position Offset is subtracted from the "Secondary Position".<br>It can only be written with the axis stopped. | UINT32 | RW | 0x00000000 (0dec) |
| 8100:19 | Gear ratio motor shaft revolutions | These parameters are used to scale all positions and speeds from the motor side to the load side of a gear unit.<br>"Gear ratio motor shaft revolutions" describes the number of motor revolutions required to achieve the number of load revolutions configured in "Gear ratio driving shaft revolutions".<br>Example: For a reduction gear in which 5 motor revolutions result in 2 load revolutions, set the parameters as follows:<br>• Motor shaft revolutions = 5<br>• Driving shaft revolutions = 2 | UINT32 | RW | 0x00000001 (1dec) |
| 8100:1A | Gear ratio driving shaft revolutions | | UINT32 | RW | 0x00000001 (1dec) |
| 8100:1B | Min position range limit | Lowest value for the position display of setpoints and actual values. If the value falls below this, an underflow to the value "Max position range limit" occurs.<br>"Min position range limit" must always be lower than "Max position range limit". | UINT32 | RW | 0x00000000 (0dec) |
| 8100:1C | Max position range limit | Highest value for the position display of setpoints and actual values. If this is exceeded, an overflow to the value "Min position range limit" occurs.<br>"Max position range limit" must always be higher than "Min position range limit". | UINT32 | RW | 0xFFFFFFFF (4294967295dec) |

##### Index 8101 FB Touch probe Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8101:0 | FB Touch probe Settings Ch.2 | | UINT8 | RO | 0x16 (22dec) |
| 8101:11 | Touch probe 1 source | Selection of the input signal for Touch probe 1.<br>Permitted values:<br>• 1: Touch probe input 1<br>• 5: Hardware zero impulse | INT16 | RW | 0x0001 (1dec) |
| 8101:12 | Touch probe 2 source | Selection of the input signal for Touch probe 2.<br>Permitted values:<br>• 2: Touch probe input 2<br>• 5: Hardware zero impulse | INT16 | RW | 0x0002 (2dec) |
| 8101:15 | Touch probe 1 position source | Selection of the position held by Touch probe 1.<br>Permitted values:<br>• 0: FB Position<br>• 1: FB Secondary Position | INT16 | RW | 0x0000 (0dec) |
| 8101:16 | Touch probe 2 position source | Selection of the position held by Touch probe 2.<br>Permitted values:<br>• 0: FB Position<br>• 1: FB Secondary Position | INT16 | RW | 0x0000 (0dec) |

##### Index 8108 FB Settings ENC Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8108:0 | FB Settings ENC Ch.2 | | UINT8 | RO | 0x13 (19dec) |
| 8108:01 | Invert feedback direction | Changes the counting direction of the encoder.<br>This parameter can be used to adapt the encoder direction of rotation to the motor direction of rotation. | BOOLEAN | RW | 0x00 (0dec) |
| 8108:12 | Encoder type | permitted values:<br>• 0: disabled<br>• 1: RS422 differential<br>• 2: TTL single ended<br>• 6: TTL single ended - input filters disabled<br>• 7: open collector | UINT16 | RW | 0x0000 (0dec) |
| 8108:13 | Encoder Increments per Revolution | Resolution of the encoder after 4-fold evaluation. | UINT32 | RW | 0x00001000 (4096dec) |

##### Index 8110 DRV Amplifier Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8110:0 | DRV Amplifier Settings Ch.2 | | UINT8 | RO | 0x73 (115dec) |
| 8110:01 | Enable TxPDOToggle | Show TxPDO Toggle in the status word (bit 10). | BOOLEAN | RW | 0x00 (0dec) |
| 8110:02 | Enable input cycle counter | 1: enabled<br>The Input cycle counter is a two-bit counter that is incremented with each process data cycle up to a maximum value of 3, after which it starts again at 0.<br>The low bit is represented in bit 10 and the high bit in bit 14 of the status word. | BOOLEAN | RW | 0x00 (0dec) |
| 8110:04 | Repeat find commutation | Repeat the commutation angle determination. (Effective for all FOC operation modes). | BOOLEAN | RW | 0x01 (1dec) |
| 8110:12 | Current loop integral time | Integral component current controller [0.1 ms]. | UINT16 | RW | 0x0032 (50dec) |
| 8110:13 | Current loop proportional gain | Proportional component current controller [0.1 V/A] | UINT16 | RW | 0x0032 (50dec) |
| 8110:14 | Velocity loop integral time | Integral component velocity controller [0.1 ms]. | UINT32 | RW | 0x0000001E (30dec) |
| 8110:15 | Velocity loop proportional gain | Proportional component velocity controller [mA/(rad/s)]. | UINT32 | RW | 0x00000096 (150dec) |
| 8110:17 | Position loop proportional gain | Proportional component position controller [1/s] | UINT32 | RW | 0x0000000A (10dec) |
| 8110:31 | Velocity limitation | Limitation of the drive speed set value [1/min]. (Only effective in the CSV and CSP controller operation modes)<br>When using a gear ratio, this parameter refers to the load side. | UINT32 | RW | 0x000186A0 (100000dec) |
| 8110:32 | Short-Circuit Brake duration max | Max. duration of armature short circuit brake. [ms] | UINT16 | RW | 0x0000 (0dec) |
| 8110:33 | Stand still window | Tolerance window for standstill monitoring [1/min] | UINT16 | RW | 0x0001 (1dec) |
| 8110:39 | Select info data 1 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x02 (2dec) |
| 8110:3A | Select info data 2 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x04 (4dec) |
| 8110:49 | Halt ramp deceleration | Halt ramp deceleration [0.1 rad / s²] | UINT32 | RW | 0x0000F570 (62832dec) |
| 8110:50 | Following error window | Following error monitoring: following error window.<br>The value 0xFFFFFFFFFF (4294967295dec) disables the following error monitoring. | UINT32 | RW | 0xFFFFFFFF (4294967295dec) |
| 8110:51 | Following error time out | Following error monitoring: timeout [ms]. | UINT16 | RW | 0x0000 (0dec) |
| 8110:52 | Fault reaction option code | permitted values:<br>• 0: Disable drive function, motor is free to rotate<br>• 1: Slow down on slow down ramp<br>• 65534dec: Short circuit brake | UINT16 | RW | 0x0001 (1dec) |
| 8110:54 | Feature bits | | UINT32 | RW | 0x00000000 (0dec) |
| 8110:57 | Position loop velocity feed forward gain | Scaling factor for velocity pre-control from the position interpolator. | UINT8 | RW | 0x64 (100dec) |
| 8110:58 | Select info data 3 | permitted values:<br>• 2: DC link voltage (mV)<br>• 4: PCB temperature (0.1 °C)<br>• 10: Digital inputs | UINT8 | RW | 0x0A (10dec) |
| 8110:59 | Error suppression mask | | UINT32 | RW | 0x00000000 (0dec) |
| 8110:62 | Position loop deadband window | Deadband window of the position controller.<br>Unit: corresponds to the process data scaling of the set position and actual positions. | UINT32 | RW | 0x00000000 (0dec) |
| 8110:63 | Find commutation time | For field-oriented control (FOC) with an incremental encoder, commutation determination is required, during which nominal current is applied to the motor. The "Find commutation time" describes the time used for this. | UINT16 | RW | 0x000A (10dec) |
| 8110:64 | Commutation type | permitted values:<br>• 16: Stepper with internal counter<br>• 17: Stepper with encoder<br>• 18: Stepper FOC with encoder | UINT8 | RW | 0x10 (16dec) |
| 8110:65 | Invert direction of rotation | Inverting the direction of rotation.<br>This parameter inverts all setpoints and actual values and is used to ensure that the motor rotates in the correct direction for the application.<br>Note: This parameter is not suitable for adapting the directions of rotation of the encoder and motor to each other. Use 8008:01 "Invert feedback direction" for this purpose. | BOOLEAN | RW | 0x00 (0dec) |
| 8110:6D | Torque feed forward gain | Internal torque pre-control: scaling factor | UINT32 | RW | 0x00000064 (100dec) |
| 8110:6E | Torque feed forward filter time | Internal torque pre-control: filter time. [0.1 ms] | UINT32 | RW | 0x0000000A (10dec) |
| 8110:6F | Torque offset | Torque offset.<br>The value is given in thousandths of the nominal current. | INT16 | RW | 0x0000 (0dec) |
| 8110:70 | Torque limitation option code | Selection of the behavior in the CST controller operation mode (“Cyclic Synchronous Torque”).<br>Permitted values:<br>• 0: VeloLimitHasNoEffect<br>• 1: TorqueMightBeReducedToZero<br>• 2: TorqueMightBeReducedToRampPosNeg<br>• 3: TorqueMightBeReducedToRampPosMaxTorqueNeg<br>• 4: TorqueMightBeReducedToMaxTorquePosNeg | INT8 | RW | 0x00 (0dec) |
| 8110:72 | Stand still torque limitation | Only valid for commutation types "Stepper with internal counter" and "Stepper with encoder" (can be set via parameter 8110:64).<br>Configures a current reduction at standstill, i.e. when the target velocity is within the "Stand still window" (parameter 8110:33).<br>The value is given in thousandths of the nominal current. | UINT16 | RW | 0x7FFF (32767dec) |
| 8110:73 | Acceleration limitation | Limits the maximum acceleration or deceleration. [0.1 rad/s²] | UINT32 | RW | 0x0000F570 (62832dec) |

##### Index 8111 DRV Motor Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8111:0 | DRV Motor Settings Ch.2 | | UINT8 | RO | 0x34 (52dec) |
| 8111:12 | Rated current | The nominal current of the motor (data sheet value). Used for scaling "Torque actual value" and "Target torque". | UINT32 | RW | 0x00000BB8 (3000dec) |
| 8111:16 | Torque constant | Force constant of the motor. | UINT32 | RW | 0x0000012C (300dec) |
| 8111:18 | Rotor moment of inertia | Moment of inertia of the motor. | UINT32 | RW | 0x000001EF (495dec) |
| 8111:19 | Winding inductance | Winding inductance. | UINT16 | RW | 0x0186 (390dec) |
| 8111:1B | Motor speed limitation | Speed limit of the motor.<br>When using a gear ratio, this limit still refers to the motor side. | UINT32 | RW | 0x000186A0 (100000dec) |
| 8111:2E | Rated speed | Nominal speed of the motor. | UINT32 | RW | 0x000003E8 (1000dec) |
| 8111:30 | Winding resistance | Winding resistance of the motor. | UINT32 | RW | 0x00000578 (1400dec) |
| 8111:31 | Voltage constant | Voltage constant of the motor. | UINT32 | RW | 0x00004E20 (20000dec) |
| 8111:33 | Motor fullsteps per revolution | Number of full motor steps per revolution. | UINT32 | RW | 0x000000C8 (200dec) |
| 8111:34 | Configured motor current | Configured motor current. If this is smaller than the "Rated current", the motor current is limited to the smaller of the two values.<br>This value is used to distribute the load between the two channels of the terminal. | UINT32 | RW | 0x00000BB8 (3000dec) |

##### Index 8112 DRV Brake Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8112:0 | DRV Brake Settings Ch.2 | | UINT8 | RO | 0x14 (20dec) |
| 8112:01 | Enable manual override | • True: The brake state is forced manually via the CoE.<br>• False: The brake is automatically controlled by the drive controller. | BOOLEAN | RW | 0x00 (0dec) |
| 8112:02 | Manual brake state | Permitted values:<br>• 0: Release<br>• 1: Apply | BIT1 | RW | 0x00 (0dec) |
| 8112:05 | Brake option | Permitted values:<br>• 0: Enable output to release brake (for brakes that are released in the energized state)<br>• 1: Disable output to release brake (for brakes that are released in a de-energized state) | BIT4 | RW | 0x00 (0dec) |
| 8112:09 | External override | Enables the brake to be released via an external hardware signal.<br>The brake can only be released via the external signal, not applied.<br>You can select which hardware input (Touch probe 1 / Touch probe 2) is used for this. It is also possible to restrict whether this configuration is always active or only in the EtherCAT states INIT/PREOP/SAFEOP, i.e. when the drive is not fully operational (e.g. during maintenance work)<br>Permitted values:<br>• 0: Disabled<br>• 2: Digital Input 1<br>• 3: Digital Input 1 (only INIT/PREOP/SAFEOP)<br>• 4: Digital Input 2<br>• 5: Digital Input 2 (only INIT/PREOP/SAFEOP) | UINT8 | RW | 0x00 (0dec) |
| 8112:11 | Release delay | Time required for the holding brake to release after the current has been applied. | UINT16 | RW | 0x0000 (0dec) |
| 8112:12 | Application delay | Time required for the holding brake to apply after the current was switched off. | UINT16 | RW | 0x0000 (0dec) |
| 8112:13 | Emergency application timeout | Time that the amplifier waits for the speed to reach the standstill limit after a stop request.<br>If the waiting time is exceeded, the holding brake is triggered, regardless of the speed.<br>Note: This parameter must be set at least to the longest time the axis needs to come to a standstill after it has been switched torque-free.<br>For vertical axes, this parameter should be set to a low value to prevent the axis or load from falling very far.<br>Unit: ms | UINT16 | RW | 0x0000 (0dec) |
| 8112:14 | Brake moment of inertia | Moment of inertia of the brake.<br>Unit: g cm² | UINT16 | RW | 0x0000 (0dec) |

##### Index 8113 DRV Filter Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8113:0 | DRV Filter Settings Ch.2 | | UINT8 | RO | 0x19 (25dec) |
| 8113:10 | Low pass frequency 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:11 | Low pass damping 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:12 | High pass frequency 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:13 | High pass damping 1 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:14 | Filter type 1 | permitted values:<br>• 0: No_Filter<br>• 1: Low_pass_filter_1_order<br>• 2: Phase_correction_filter_1_order<br>• 3: Low_pass_filter_2_order<br>• 4: Phase_correction_filter_2_order<br>• 5: Notch_filter | INT16 | RW | 0x0000 (0dec) |
| 8113:15 | Low pass frequency 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:16 | Low pass damping 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:17 | High pass frequency 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:18 | High pass damping 2 | | REAL32 | RW | 0x00000000 (0dec) |
| 8113:19 | Filter type 2 | permitted values:<br>• 0: No_Filter<br>• 1: Low_pass_filter_1_order<br>• 2: Phase_correction_filter_1_order<br>• 3: Low_pass_filter_2_order<br>• 4: Phase_correction_filter_2_order<br>• 5: Notch_filter | INT16 | RW | 0x0000 (0dec) |

##### Index 811F DRV Vendor data Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 811F:0 | DRV Vendor data Ch.2 | | UINT8 | RO | 0x1C (28dec) |
| 811F:12 | Amplifier rated current | Maximum current per channel (without fan). | UINT32 | RW | 0x00001388 (5000dec) |
| 811F:14 | Amplifier overcurrent threshold | Switching threshold for the overcurrent switch-off. | UINT32 | RW | 0x00002710 (10000dec) |
| 811F:15 | Max rotary field frequency | | UINT16 | RW | 0x0257 (599dec) |
| 811F:17 | Amplifier rated current with fan | Maximum current per channel (with fan) | UINT32 | RW | 0x00001770 (6000dec) |
| 811F:18 | Vendor feature bits | | UINT32 | RW | 0x00000000 (0dec) |
| 811F:1A | Amplifier Rated Sum Current | Maximum sum current for all channels (without fan) | UINT32 | RW | 0x00001770 (6000dec) |
| 811F:1C | Amplifier Rated Sum Current with Fan | Maximum sum current for all channels (with fan) | UINT32 | RW | 0x00002710 (10000dec) |

##### Index 8160 DMC Settings Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8160:0 | DMC Settings Ch.2 | | UINT8 | RO | 0x17 (23dec) |
| 8160:07 | Emergency deceleration | Deceleration for the emergency stop ramp. (In ms from motor nominal speed to standstill)<br>Unit: 1 ms | UINT16 | RW | 0x0064 (100dec) |
| 8160:08 | Calibration position | If homing is successful, the "Actual position" is set to this value. | INT64 | RW | |
| 8160:09 | Calibration velocity (towards plc cam) | Velocity when hitting the cam in 10000ths of the motor nominal speed. | INT16 | RW | 0x0064 (100dec) |
| 8160:0A | Calibration Velocity (off plc cam) | Velocity when driving off the cam in 10000ths of the motor nominal speed. | INT16 | RW | 0x000A (10dec) |
| 8160:0E | Modulo factor | Feedback increments for one mechanical revolution. | INT64 | RW | |
| 8160:12 | Block calibration torque limit | Torque limitation for approaching the end stop. In parts per thousand of the nominal motor current. | UINT16 | RW | 0x0064 (100dec) |
| 8160:13 | Block calibration stop distance | After reaching the calibration position, the axis moves out of the end position by this distance. | INT64 | RW | |
| 8160:14 | Block calibration lag threshold | When this following error is exceeded, the axis is in the end position. | INT64 | RW | |
| 8160:15 | Target position window | Target position window:<br>The In-Target bit is set when the axis is within this window for at least the time set in 8160:16. | INT64 | RW | |
| 8160:16 | Target position monitor time | see 8160:15<br>Unit: ms | UINT16 | RW | 0x0014 (20dec) |
| 8160:17 | Target position timeout | When the setpoint generator has reached its end position and the axis is not in the target window after this time has elapsed, the task is terminated and the in-target bit is not set.<br>Unit: ms | UINT16 | RW | 0x1770 (6000dec) |

##### Index 8161 DMC Features Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 8161:0 | DMC Features Ch.2 | | UINT8 | RO | 0x1B (27dec) |
| 8161:13 | Invert calibration cam search direction | Invert the direction of movement to search for the limit switch.<br>Default: FALSE = search with positive direction of rotation. | BOOLEAN | RW | 0x00 (0dec) |
| 8161:14 | Invert sync impulse search direction | Invert the direction of rotation to exit the limit switch.<br>Default: TRUE = exit in negative direction of rotation. | BOOLEAN | RW | 0x01 (1dec) |
| 8161:19 | Calibration cam source | Source for the reference switch.<br>• 0: Input 1<br>• 1: Input 2 | UINT8 | RW | 0x00 (0dec) |
| 8161:1A | Calibration cam active level | State of the reference switch in the actuated state.<br>• 0: Hi<br>• 1: Low | UINT8 | RW | 0x00 (0dec) |
| 8161:1B | Latch source | Source for the latch unit.<br>• 0: Input 1<br>• 1: Input 2 | UINT8 | RW | 0x00 (0dec) |

##### Index F800 DRV Amplifier Settings

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F800:0 | DRV Amplifier Settings | | UINT8 | RO | 0x18 (24dec) |
| F800:10 | Nominal DC link voltage | Nominal DC link voltage. | UINT32 | RW | 0x0000BB80 (48000dec) |
| F800:11 | Min DC link voltage | Min. DC link voltage.<br>If the value falls below this, a drive error is triggered or an inactive axis cannot be switched on. | UINT32 | RW | 0x00001A90 (6800dec) |
| F800:12 | Max DC link voltage | Max. DC link voltage.<br>If this value is exceeded, a drive error is triggered or an inactive axis cannot be switched on. | UINT32 | RW | 0x0000EA60 (60000dec) |
| F800:15 | Amplifier Temperature warn level | Amplifier temperature warning threshold. | UINT16 | RW | 0x0320 (800dec) |
| F800:16 | Amplifier Temperature error level | Amplifier temperature error threshold. | UINT16 | RW | 0x03E8 (1000dec) |
| F800:17 | Feature bits | | UINT32 | RW | 0x00000000 (0dec) |
| F800:18 | Fan Configuration | An external fan can be used to increase the permissible motor current for the terminal.<br>Permitted values:<br>• 0: no fan<br>• 1: fan installed | UINT8 | RW | 0x00 (0dec) |

#### 8.1.2 Diagnostic objects

##### Index 9010 DRV Info data Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 9010:0 | DRV Info data Ch.1 | | UINT8 | RO | 0x28 (40dec) |
| 9010:13 | Supported drive modes | | UINT32 | RO | 0x00000000 (0dec) |
| 9010:14 | Velocity encoder resolution | | UINT32 | RO | 0x00000000 (0dec) |
| 9010:15 | Position encoder resolution increments | | UINT32 | RO | 0x00000000 (0dec) |
| 9010:16 | Position encoder resolution revolutions | | UINT32 | RO | 0x00000000 (0dec) |
| 9010:17 | Cogging compensation supported | | BOOLEAN | RO | 0x00 (0dec) |
| 9010:27 | Output stage safety state | permitted values:<br>• 0: safe_state<br>• 1: ready_state | UINT8 | RO | 0x00 (0dec) |
| 9010:28 | Actual motor brake state | permitted values:<br>• 0: Motor brake applied<br>• 1: Motor brake released | UINT8 | RO | 0x00 (0dec) |

##### Index 9110 DRV Info data Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 9110:0 | DRV Info data Ch.2 | | UINT8 | RO | 0x28 (40dec) |
| 9110:13 | Supported drive modes | | UINT32 | RO | 0x00000000 (0dec) |
| 9110:14 | Velocity encoder resolution | | UINT32 | RO | 0x00000000 (0dec) |
| 9110:15 | Position encoder resolution increments | | UINT32 | RO | 0x00000000 (0dec) |
| 9110:16 | Position encoder resolution revolutions | | UINT32 | RO | 0x00000000 (0dec) |
| 9110:17 | Cogging compensation supported | | BOOLEAN | RO | 0x00 (0dec) |
| 9110:27 | Output stage safety state | permitted values:<br>• 0: safe_state<br>• 1: ready_state | UINT8 | RO | 0x00 (0dec) |
| 9110:28 | Actual motor brake state | permitted values:<br>• 0: Motor brake applied<br>• 1: Motor brake released | UINT8 | RO | 0x00 (0dec) |

##### Index F900 DRV Info data

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F900:0 | DRV Info data | | UINT8 | RO | 0x13 (19dec) |
| F900:11 | Amplifier temperature | | INT16 | RO | 0x0000 (0dec) |
| F900:12 | DC link voltage | | UINT32 | RO | 0x00000000 (0dec) |
| F900:13 | Supply voltage Up | | UINT32 | RO | 0x00000000 (0dec) |

##### Index F913 DRV Device Info data

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F913:0 | DRV Device Info data | | UINT8 | RO | 0x04 (4dec) |
| F913:01 | HW config | | STRING | RO | |
| F913:02 | FB config | | STRING | RO | |
| F913:03 | FW info | | STRING | RO | |
| F913:04 | DMC version | | STRING | RO | |

#### 8.1.3 Standard objects

##### Index 1000 Device type

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1000:0 | Device type | Device type of the EtherCAT slave: the Lo-Word contains the used CoE profile (5001). The Hi-Word contains the module profile according to the modular device profile. | UINT32 | RO | 0x00001389 (5001dec) |

##### Index 1008 Device name

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1008:0 | Device name | Device name of the EtherCAT slave | STRING | RO | EL7062‑0000 |

##### Index 1009 Hardware version

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1009:0 | Hardware version | Hardware version of the EtherCAT slave | STRING | RO | |

##### Index 100A Software version

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 100A:0 | Software version | Firmware version of the EtherCAT slave | STRING | RO | 01 |

##### Index 100B Bootloader version

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 100B:0 | Bootloader version | | STRING | RO | N/A |

##### Index 1011 Restore default parameters

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1011:0 | Restore default parameters | Restore default parameters | UINT8 | RO | 0x01 (1dec) |
| 1011:01 | SubIndex 001 | If this object is set to "0x64616F6C" in the set value dialog, all backup objects are reset to their delivery state. | UINT32 | RW | 0x00000000 (0dec) |

##### Index 1018 Identity

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1018:0 | Identity | Information for identifying the slave | UINT8 | RO | 0x04 (4dec) |
| 1018:01 | Vendor ID | Vendor ID of the EtherCAT slave | UINT32 | RO | 0x00000002 (2dec) |
| 1018:02 | Product code | Product code of the EtherCAT slave | UINT32 | RO | 0x1B963052 (462827602dec) |
| 1018:03 | Revision | Revision number of the EtherCAT slave; the Low Word (bit 0-15) indicates the special terminal number, the High Word (bit 16-31) refers to the device description | UINT32 | RO | 0x00000000 (0dec) |
| 1018:04 | Serial number | Serial number of the EtherCAT slave; the Low Byte (bit 0-7) of the Low Word contains the year of production, the High Byte (bit 8-15) of the Low Word contains the week of production, the High Word (bit 16-31) is 0 | UINT32 | RO | 0x00000000 (0dec) |

##### Index 10E2 Manufacturer-specific Identification Code

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 10E2:0 | Manufacturer-specific Identification Code | | UINT8 | RO | 0x01 (1dec) |
| 10E2:01 | SubIndex 001 | | STRING | RO | |

##### Index 10F0 Backup parameter handling

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 10F0:01 | Checksum | Checksum across all backup entries of the EtherCAT slave | UINT32 | RO | 0x00000000 (0dec) |

##### Index 10F3 Diagnosis History

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 10F3:0 | Diagnosis History | | UINT8 | RO | 0x37 (55dec) |
| 10F3:01 | Maximum Messages | | UINT8 | RO | 0x00 (0dec) |
| 10F3:02 | Newest Message | | UINT8 | RO | 0x00 (0dec) |
| 10F3:03 | Newest Acknowledged Message | | UINT8 | RW | 0x00 (0dec) |
| 10F3:04 | New Messages Available | | BOOLEAN | RO | 0x00 (0dec) |
| 10F3:05 | Flags | | UINT16 | RW | 0x0000 (0dec) |
| 10F3:06 | Diagnosis Message 001 | | OCTET-STRING[32] | RO | {0} |
| 10F3:07 | Diagnosis Message 002 | | OCTET-STRING[32] | RO | {0} |
| 10F3:08 | Diagnosis Message 003 | | OCTET-STRING[32] | RO | {0} |
| … | … | … | … | … | … |
| 10F3:35 | Diagnosis Message 048 | | OCTET-STRING[32] | RO | {0} |
| 10F3:36 | Diagnosis Message 049 | | OCTET-STRING[32] | RO | {0} |
| 10F3:37 | Diagnosis Message 050 | | OCTET-STRING[32] | RO | {0} |

##### Index 10F8 Timestamp Object

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 10F8:0 | Timestamp Object | | UINT64 | RO | |

##### Index 1460 DMC RxPDO-Par Outputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1460:0 | DMC RxPDO-Par Outputs Ch.1 | PDO Parameter RxPDO 97 | UINT8 | RO | 0x06 (6dec) |
| 1460:06 | Exclude RxPDOs | Specifies the RxPDOs (index of RxPDO mapping objects) that must not be transferred together with RxPDO 97 | OCTET-STRING[2] | RO | 61 16 |

##### Index 1461 DMC RxPDO-Par Outputs 32 Bit Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1461:0 | DMC RxPDO-Par Outputs 32 Bit Ch.1 | PDO Parameter RxPDO 98 | UINT8 | RO | 0x06 (6dec) |
| 1461:06 | Exclude RxPDOs | Specifies the RxPDOs (index of RxPDO mapping objects) that must not be transferred together with RxPDO 98 | OCTET-STRING[2] | RO | 60 16 |

##### Index 14E0 DMC RxPDO-Par Outputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 14E0:0 | DMC RxPDO-Par Outputs Ch.2 | PDO Parameter RxPDO 225 | UINT8 | RO | 0x06 (6dec) |
| 14E0:06 | Exclude RxPDOs | Specifies the RxPDOs (index of RxPDO mapping objects) that must not be transferred together with RxPDO 225 | OCTET-STRING[2] | RO | E1 16 |

##### Index 14E1 DMC RxPDO-Par Outputs 32 Bit Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 14E1:0 | DMC RxPDO-Par Outputs 32 Bit Ch.2 | PDO Parameter RxPDO 226 | UINT8 | RO | 0x06 (6dec) |
| 14E1:06 | Exclude RxPDOs | Specifies the RxPDOs (index of RxPDO mapping objects) that must not be transferred together with RxPDO 226 | OCTET-STRING[2] | RO | E0 16 |

##### Index 1600 DRV RxPDO-Map Controlword Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1600:0 | DRV RxPDO-Map Controlword Ch.1 | PDO Mapping RxPDO 1 | UINT8 | RO | 0x01 (1dec) |
| 1600:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x01 (Controlword)) | UINT32 | RO | 0x7010:01, 16 |

##### Index 1601 DRV RxPDO-Map Target velocity Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1601:0 | DRV RxPDO-Map Target velocity Ch.1 | PDO Mapping RxPDO 2 | UINT8 | RO | 0x01 (1dec) |
| 1601:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x06 (Target velocity)) | UINT32 | RO | 0x7010:06, 32 |

##### Index 1602 DRV RxPDO-Map Target torque Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1602:0 | DRV RxPDO-Map Target torque Ch.1 | PDO Mapping RxPDO 3 | UINT8 | RO | 0x01 (1dec) |
| 1602:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x09 (Target torque)) | UINT32 | RO | 0x7010:09, 16 |

##### Index 1603 DRV RxPDO-Map Commutation angle Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1603:0 | DRV RxPDO-Map Commutation angle Ch.1 | PDO Mapping RxPDO 4 | UINT8 | RO | 0x01 (1dec) |
| 1603:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x0E (Commutation angle)) | UINT32 | RO | 0x7010:0E, 16 |

##### Index 1604 DRV RxPDO-Map Torque limitation Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1604:0 | DRV RxPDO-Map Torque limitation Ch.1 | PDO Mapping RxPDO 5 | UINT8 | RO | 0x01 (1dec) |
| 1604:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x0B (Torque limitation)) | UINT32 | RO | 0x7010:0B, 16 |

##### Index 1605 DRV RxPDO-Map Torque offset Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1605:0 | DRV RxPDO-Map Torque offset Ch.1 | PDO Mapping RxPDO 6 | UINT8 | RO | 0x01 (1dec) |
| 1605:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x0A (Torque offset)) | UINT32 | RO | 0x7010:0A, 16 |

##### Index 1606 DRV RxPDO-Map Target position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1606:0 | DRV RxPDO-Map Target position Ch.1 | PDO Mapping RxPDO 7 | UINT8 | RO | 0x01 (1dec) |
| 1606:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x05 (Target position)) | UINT32 | RO | 0x7010:05, 32 |

##### Index 1607 FB RxPDO-Map Touch probe control Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1607:0 | FB RxPDO-Map Touch probe control Ch.1 | PDO Mapping RxPDO 8 | UINT8 | RO | 0x0C (12dec) |
| 1607:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x01 (TP1 Enable)) | UINT32 | RO | 0x7001:01, 1 |
| 1607:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x02 (TP1 Continous)) | UINT32 | RO | 0x7001:02, 1 |
| 1607:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x03 (TP1 Trigger mode)) | UINT32 | RO | 0x7001:03, 2 |
| 1607:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x05 (TP1 Enable pos edge)) | UINT32 | RO | 0x7001:05, 1 |
| 1607:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x06 (TP1 Enable neg edge)) | UINT32 | RO | 0x7001:06, 1 |
| 1607:06 | SubIndex 006 | 6. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |
| 1607:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x09 (TP2 Enable)) | UINT32 | RO | 0x7001:09, 1 |
| 1607:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x0A (TP2 Continous)) | UINT32 | RO | 0x7001:0A, 1 |
| 1607:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x0B (TP2 Trigger mode)) | UINT32 | RO | 0x7001:0B, 2 |
| 1607:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x0D (TP2 Enable pos edge)) | UINT32 | RO | 0x7001:0D, 1 |
| 1607:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x7001 (FB Touch probe outputs Ch.1), entry 0x0E (TP2 Enable neg edge)) | UINT32 | RO | 0x7001:0E, 1 |
| 1607:0C | SubIndex 012 | 12. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |

##### Index 1608 DRV RxPDO-Map Modes of operation Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1608:0 | DRV RxPDO-Map Modes of operation Ch.1 | PDO Mapping RxPDO 9 | UINT8 | RO | 0x01 (1dec) |
| 1608:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x03 (Modes of operation)) | UINT32 | RO | 0x7010:03, 8 |

##### Index 1609 DRV RxPDO-Map Velocity offset Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1609:0 | DRV RxPDO-Map Velocity offset Ch.1 | PDO Mapping RxPDO 10 | UINT8 | RO | 0x01 (1dec) |
| 1609:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x0F (Velocity offset)) | UINT32 | RO | 0x7010:0F, 32 |

##### Index 160A DRV RxPDO-Map Positive torque limit value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 160A:0 | DRV RxPDO-Map Positive torque limit value Ch.1 | PDO Mapping RxPDO 11 | UINT8 | RO | 0x01 (1dec) |
| 160A:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x10 (Positive torque limit value)) | UINT32 | RO | 0x7010:10, 16 |

##### Index 160B DRV RxPDO-Map Negative torque limit value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 160B:0 | DRV RxPDO-Map Negative torque limit value Ch.1 | PDO Mapping RxPDO 12 | UINT8 | RO | 0x01 (1dec) |
| 160B:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x11 (Negative torque limit value)) | UINT32 | RO | 0x7010:11, 16 |

##### Index 160C DRV RxPDO-Map Low velocity limit value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 160C:0 | DRV RxPDO-Map Low velocity limit value Ch.1 | PDO Mapping RxPDO 13 | UINT8 | RO | 0x01 (1dec) |
| 160C:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x12 (Low velocity limit value)) | UINT32 | RO | 0x7010:12, 32 |

##### Index 160D DRV RxPDO-Map High velocity limit value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 160D:0 | DRV RxPDO-Map High velocity limit value Ch.1 | PDO Mapping RxPDO 14 | UINT8 | RO | 0x01 (1dec) |
| 160D:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7010 (DRV Outputs Ch.1), entry 0x13 (High velocity limit value)) | UINT32 | RO | 0x7010:13, 32 |

##### Index 1660 DMC RxPDO-Map Outputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1660:0 | DMC RxPDO-Map Outputs Ch.1 | PDO Mapping RxPDO 97 | UINT8 | RO | 0x12 (18dec) |
| 1660:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1660:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x02 (DMC__FeedbackControl__Enable latch extern on positive edge)) | UINT32 | RO | 0x7060:02, 1 |
| 1660:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x03 (DMC__FeedbackControl__Set counter)) | UINT32 | RO | 0x7060:03, 1 |
| 1660:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x04 (DMC__FeedbackControl__Enable latch extern on negative edge)) | UINT32 | RO | 0x7060:04, 1 |
| 1660:05 | SubIndex 005 | 5. PDO Mapping entry (12 bits align) | UINT32 | RO | 0x0000:00, 12 |
| 1660:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x11 (DMC__DriveControl__Enable)) | UINT32 | RO | 0x7060:11, 1 |
| 1660:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x12 (DMC__DriveControl__Reset)) | UINT32 | RO | 0x7060:12, 1 |
| 1660:08 | SubIndex 008 | 8. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 1660:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x21 (DMC__PositioningControl__Execute)) | UINT32 | RO | 0x7060:21, 1 |
| 1660:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x22 (DMC__PositioningControl__Emergency stop)) | UINT32 | RO | 0x7060:22, 1 |
| 1660:0B | SubIndex 011 | 11. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 1660:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x31 (DMC__Set counter value)) | UINT32 | RO | 0x7060:31, 64 |
| 1660:0D | SubIndex 013 | 13. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x32 (DMC__Target position)) | UINT32 | RO | 0x7060:32, 64 |
| 1660:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x33 (DMC__Target velocity)) | UINT32 | RO | 0x7060:33, 16 |
| 1660:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x34 (DMC__Start type)) | UINT32 | RO | 0x7060:34, 16 |
| 1660:10 | SubIndex 016 | 16. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x35 (DMC__Target acceleration)) | UINT32 | RO | 0x7060:35, 16 |
| 1660:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x36 (DMC__Target deceleration)) | UINT32 | RO | 0x7060:36, 16 |
| 1660:12 | SubIndex 018 | 18. PDO Mapping entry (80 bits align) | UINT32 | RO | 0x0000:00, 80 |

##### Index 1661 DMC RxPDO-Map Outputs 32 Bit Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1661:0 | DMC RxPDO-Map Outputs 32 Bit Ch.1 | PDO Mapping RxPDO 98 | UINT8 | RO | 0x14 (20dec) |
| 1661:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1661:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x02 (DMC__FeedbackControl__Enable latch extern on positive edge)) | UINT32 | RO | 0x7060:02, 1 |
| 1661:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x03 (DMC__FeedbackControl__Set counter)) | UINT32 | RO | 0x7060:03, 1 |
| 1661:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x04 (DMC__FeedbackControl__Enable latch extern on negative edge)) | UINT32 | RO | 0x7060:04, 1 |
| 1661:05 | SubIndex 005 | 5. PDO Mapping entry (12 bits align) | UINT32 | RO | 0x0000:00, 12 |
| 1661:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x11 (DMC__DriveControl__Enable)) | UINT32 | RO | 0x7060:11, 1 |
| 1661:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x12 (DMC__DriveControl__Reset)) | UINT32 | RO | 0x7060:12, 1 |
| 1661:08 | SubIndex 008 | 8. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 1661:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x21 (DMC__PositioningControl__Execute)) | UINT32 | RO | 0x7060:21, 1 |
| 1661:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x22 (DMC__PositioningControl__Emergency stop)) | UINT32 | RO | 0x7060:22, 1 |
| 1661:0B | SubIndex 011 | 11. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 1661:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x31 (DMC__Set counter value)) | UINT32 | RO | 0x7060:31, 32 |
| 1661:0D | SubIndex 013 | 13. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1661:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x32 (DMC__Target position)) | UINT32 | RO | 0x7060:32, 32 |
| 1661:0F | SubIndex 015 | 15. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1661:10 | SubIndex 016 | 16. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x33 (DMC__Target velocity)) | UINT32 | RO | 0x7060:33, 16 |
| 1661:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x34 (DMC__Start type)) | UINT32 | RO | 0x7060:34, 16 |
| 1661:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x35 (DMC__Target acceleration)) | UINT32 | RO | 0x7060:35, 16 |
| 1661:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x7060 (DMC Outputs Ch.1), entry 0x36 (DMC__Target deceleration)) | UINT32 | RO | 0x7060:36, 16 |
| 1661:14 | SubIndex 020 | 20. PDO Mapping entry (80 bits align) | UINT32 | RO | 0x0000:00, 80 |

##### Index 1680 DRV RxPDO-Map Controlword Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1680:0 | DRV RxPDO-Map Controlword Ch.2 | PDO Mapping RxPDO 129 | UINT8 | RO | 0x01 (1dec) |
| 1680:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x01 (Controlword)) | UINT32 | RO | 0x7110:01, 16 |

##### Index 1681 DRV RxPDO-Map Target velocity Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1681:0 | DRV RxPDO-Map Target velocity Ch.2 | PDO Mapping RxPDO 130 | UINT8 | RO | 0x01 (1dec) |
| 1681:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x06 (Target velocity)) | UINT32 | RO | 0x7110:06, 32 |

##### Index 1682 DRV RxPDO-Map Target torque Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1682:0 | DRV RxPDO-Map Target torque Ch.2 | PDO Mapping RxPDO 131 | UINT8 | RO | 0x01 (1dec) |
| 1682:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x09 (Target torque)) | UINT32 | RO | 0x7110:09, 16 |

##### Index 1683 DRV RxPDO-Map Commutation angle Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1683:0 | DRV RxPDO-Map Commutation angle Ch.2 | PDO Mapping RxPDO 132 | UINT8 | RO | 0x01 (1dec) |
| 1683:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x0E (Commutation angle)) | UINT32 | RO | 0x7110:0E, 16 |

##### Index 1684 DRV RxPDO-Map Torque limitation Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1684:0 | DRV RxPDO-Map Torque limitation Ch.2 | PDO Mapping RxPDO 133 | UINT8 | RO | 0x01 (1dec) |
| 1684:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x0B (Torque limitation)) | UINT32 | RO | 0x7110:0B, 16 |

##### Index 1685 DRV RxPDO-Map Torque offset Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1685:0 | DRV RxPDO-Map Torque offset Ch.2 | PDO Mapping RxPDO 134 | UINT8 | RO | 0x01 (1dec) |
| 1685:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x0A (Torque offset)) | UINT32 | RO | 0x7110:0A, 16 |

##### Index 1686 DRV RxPDO-Map Target position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1686:0 | DRV RxPDO-Map Target position Ch.2 | PDO Mapping RxPDO 135 | UINT8 | RO | 0x01 (1dec) |
| 1686:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x05 (Target position)) | UINT32 | RO | 0x7110:05, 32 |

##### Index 1687 FB RxPDO-Map Touch probe control Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1687:0 | FB RxPDO-Map Touch probe control Ch.2 | PDO Mapping RxPDO 136 | UINT8 | RO | 0x0C (12dec) |
| 1687:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x01 (TP1 Enable)) | UINT32 | RO | 0x7101:01, 1 |
| 1687:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x02 (TP1 Continous)) | UINT32 | RO | 0x7101:02, 1 |
| 1687:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x03 (TP1 Trigger mode)) | UINT32 | RO | 0x7101:03, 2 |
| 1687:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x05 (TP1 Enable pos edge)) | UINT32 | RO | 0x7101:05, 1 |
| 1687:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x06 (TP1 Enable neg edge)) | UINT32 | RO | 0x7101:06, 1 |
| 1687:06 | SubIndex 006 | 6. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |
| 1687:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x09 (TP2 Enable)) | UINT32 | RO | 0x7101:09, 1 |
| 1687:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x0A (TP2 Continous)) | UINT32 | RO | 0x7101:0A, 1 |
| 1687:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x0B (TP2 Trigger mode)) | UINT32 | RO | 0x7101:0B, 2 |
| 1687:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x0D (TP2 Enable pos edge)) | UINT32 | RO | 0x7101:0D, 1 |
| 1687:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x7101 (FB Touch probe outputs Ch.2), entry 0x0E (TP2 Enable neg edge)) | UINT32 | RO | 0x7101:0E, 1 |
| 1687:0C | SubIndex 012 | 12. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |

##### Index 1688 DRV RxPDO-Map Modes of operation Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1688:0 | DRV RxPDO-Map Modes of operation Ch.2 | PDO Mapping RxPDO 137 | UINT8 | RO | 0x01 (1dec) |
| 1688:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x03 (Modes of operation)) | UINT32 | RO | 0x7110:03, 8 |

##### Index 1689 DRV RxPDO-Map Velocity offset Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1689:0 | DRV RxPDO-Map Velocity offset Ch.2 | PDO Mapping RxPDO 138 | UINT8 | RO | 0x01 (1dec) |
| 1689:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x0F (Velocity offset)) | UINT32 | RO | 0x7110:0F, 32 |

##### Index 168A DRV RxPDO-Map Positive torque limit value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 168A:0 | DRV RxPDO-Map Positive torque limit value Ch.2 | PDO Mapping RxPDO 139 | UINT8 | RO | 0x01 (1dec) |
| 168A:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x10 (Positive torque limit value)) | UINT32 | RO | 0x7110:10, 16 |

##### Index 168B DRV RxPDO-Map Negative torque limit value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 168B:0 | DRV RxPDO-Map Negative torque limit value Ch.2 | PDO Mapping RxPDO 140 | UINT8 | RO | 0x01 (1dec) |
| 168B:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x11 (Negative torque limit value)) | UINT32 | RO | 0x7110:11, 16 |

##### Index 168C DRV RxPDO-Map Low velocity limit value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 168C:0 | DRV RxPDO-Map Low velocity limit value Ch.2 | PDO Mapping RxPDO 141 | UINT8 | RO | 0x01 (1dec) |
| 168C:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x12 (Low velocity limit value)) | UINT32 | RO | 0x7110:12, 32 |

##### Index 168D DRV RxPDO-Map High velocity limit value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 168D:0 | DRV RxPDO-Map High velocity limit value Ch.2 | PDO Mapping RxPDO 142 | UINT8 | RO | 0x01 (1dec) |
| 168D:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x7110 (DRV Outputs Ch.2), entry 0x13 (High velocity limit value)) | UINT32 | RO | 0x7110:13, 32 |

##### Index 16E0 DMC RxPDO-Map Outputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 16E0:0 | DMC RxPDO-Map Outputs Ch.2 | PDO Mapping RxPDO 225 | UINT8 | RO | 0x12 (18dec) |
| 16E0:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 16E0:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x02 (DMC__FeedbackControl__Enable latch extern on positive edge)) | UINT32 | RO | 0x7160:02, 1 |
| 16E0:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x03 (DMC__FeedbackControl__Set counter)) | UINT32 | RO | 0x7160:03, 1 |
| 16E0:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x04 (DMC__FeedbackControl__Enable latch extern on negative edge)) | UINT32 | RO | 0x7160:04, 1 |
| 16E0:05 | SubIndex 005 | 5. PDO Mapping entry (12 bits align) | UINT32 | RO | 0x0000:00, 12 |
| 16E0:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x11 (DMC__DriveControl__Enable)) | UINT32 | RO | 0x7160:11, 1 |
| 16E0:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x12 (DMC__DriveControl__Reset)) | UINT32 | RO | 0x7160:12, 1 |
| 16E0:08 | SubIndex 008 | 8. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 16E0:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x21 (DMC__PositioningControl__Execute)) | UINT32 | RO | 0x7160:21, 1 |
| 16E0:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x22 (DMC__PositioningControl__Emergency stop)) | UINT32 | RO | 0x7160:22, 1 |
| 16E0:0B | SubIndex 011 | 11. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 16E0:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x31 (DMC__Set counter value)) | UINT32 | RO | 0x7160:31, 64 |
| 16E0:0D | SubIndex 013 | 13. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x32 (DMC__Target position)) | UINT32 | RO | 0x7160:32, 64 |
| 16E0:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x33 (DMC__Target velocity)) | UINT32 | RO | 0x7160:33, 16 |
| 16E0:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x34 (DMC__Start type)) | UINT32 | RO | 0x7160:34, 16 |
| 16E0:10 | SubIndex 016 | 16. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x35 (DMC__Target acceleration)) | UINT32 | RO | 0x7160:35, 16 |
| 16E0:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x36 (DMC__Target deceleration)) | UINT32 | RO | 0x7160:36, 16 |
| 16E0:12 | SubIndex 018 | 18. PDO Mapping entry (80 bits align) | UINT32 | RO | 0x0000:00, 80 |

##### Index 16E1 DMC RxPDO-Map Outputs 32 Bit Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 16E1:0 | DMC RxPDO-Map Outputs 32 Bit Ch.2 | PDO Mapping RxPDO 226 | UINT8 | RO | 0x14 (20dec) |
| 16E1:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 16E1:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x02 (DMC__FeedbackControl__Enable latch extern on positive edge)) | UINT32 | RO | 0x7160:02, 1 |
| 16E1:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x03 (DMC__FeedbackControl__Set counter)) | UINT32 | RO | 0x7160:03, 1 |
| 16E1:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x04 (DMC__FeedbackControl__Enable latch extern on negative edge)) | UINT32 | RO | 0x7160:04, 1 |
| 16E1:05 | SubIndex 005 | 5. PDO Mapping entry (12 bits align) | UINT32 | RO | 0x0000:00, 12 |
| 16E1:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x11 (DMC__DriveControl__Enable)) | UINT32 | RO | 0x7160:11, 1 |
| 16E1:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x12 (DMC__DriveControl__Reset)) | UINT32 | RO | 0x7160:12, 1 |
| 16E1:08 | SubIndex 008 | 8. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 16E1:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x21 (DMC__PositioningControl__Execute)) | UINT32 | RO | 0x7160:21, 1 |
| 16E1:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x22 (DMC__PositioningControl__Emergency stop)) | UINT32 | RO | 0x7160:22, 1 |
| 16E1:0B | SubIndex 011 | 11. PDO Mapping entry (14 bits align) | UINT32 | RO | 0x0000:00, 14 |
| 16E1:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x31 (DMC__Set counter value)) | UINT32 | RO | 0x7160:31, 32 |
| 16E1:0D | SubIndex 013 | 13. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 16E1:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x32 (DMC__Target position)) | UINT32 | RO | 0x7160:32, 32 |
| 16E1:0F | SubIndex 015 | 15. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 16E1:10 | SubIndex 016 | 16. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x33 (DMC__Target velocity)) | UINT32 | RO | 0x7160:33, 16 |
| 16E1:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x34 (DMC__Start type)) | UINT32 | RO | 0x7160:34, 16 |
| 16E1:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x35 (DMC__Target acceleration)) | UINT32 | RO | 0x7160:35, 16 |
| 16E1:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x7160 (DMC Outputs Ch.2), entry 0x36 (DMC__Target deceleration)) | UINT32 | RO | 0x7160:36, 16 |
| 16E1:14 | SubIndex 020 | 20. PDO Mapping entry (80 bits align) | UINT32 | RO | 0x0000:00, 80 |

##### Index 1860 DMC TxPDO-Par Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1860:0 | DMC TxPDO-Par Inputs Ch.1 | PDO parameter TxPDO 97 | UINT8 | RO | 0x06 (6dec) |
| 1860:06 | Exclude TxPDOs | Specifies the TxPDOs (index of TxPDO mapping objects) that must not be transferred together with TxPDO 97 | OCTET-STRING[2] | RO | 61 1A |

##### Index 1861 DMC TxPDO-Par Inputs 32 Bit Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1861:0 | DMC TxPDO-Par Inputs 32 Bit Ch.1 | PDO parameter TxPDO 98 | UINT8 | RO | 0x06 (6dec) |
| 1861:06 | Exclude TxPDOs | Specifies the TxPDOs (index of TxPDO mapping objects) that must not be transferred together with TxPDO 98 | OCTET-STRING[2] | RO | 60 1A |

##### Index 18E0 DMC TxPDO-Par Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 18E0:0 | DMC TxPDO-Par Inputs Ch.2 | PDO parameter TxPDO 225 | UINT8 | RO | 0x06 (6dec) |
| 18E0:06 | Exclude TxPDOs | Specifies the TxPDOs (index of TxPDO mapping objects) that must not be transferred together with TxPDO 225 | OCTET-STRING[2] | RO | E1 1A |

##### Index 18E1 DMC TxPDO-Par Inputs 32 Bit Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 18E1:0 | DMC TxPDO-Par Inputs 32 Bit Ch.2 | PDO parameter TxPDO 226 | UINT8 | RO | 0x06 (6dec) |
| 18E1:06 | Exclude TxPDOs | Specifies the TxPDOs (index of TxPDO mapping objects) that must not be transferred together with TxPDO 226 | OCTET-STRING[2] | RO | E0 1A |

##### Index 1A00 FB TxPDO-Map Position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A00:0 | FB TxPDO-Map Position Ch.1 | PDO Mapping TxPDO 1 | UINT8 | RO | 0x01 (1dec) |
| 1A00:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6000 (FB Inputs Ch.1), entry 0x11 (Position)) | UINT32 | RO | 0x6000:11, 32 |

##### Index 1A01 DRV TxPDO-Map Statusword Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A01:0 | DRV TxPDO-Map Statusword Ch.1 | PDO Mapping TxPDO 2 | UINT8 | RO | 0x01 (1dec) |
| 1A01:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x01 (Statusword)) | UINT32 | RO | 0x6010:01, 16 |

##### Index 1A02 DRV TxPDO-Map Velocity actual value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A02:0 | DRV TxPDO-Map Velocity actual value Ch.1 | PDO Mapping TxPDO 3 | UINT8 | RO | 0x01 (1dec) |
| 1A02:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x07 (Velocity actual value)) | UINT32 | RO | 0x6010:07, 32 |

##### Index 1A03 DRV TxPDO-Map Torque actual value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A03:0 | DRV TxPDO-Map Torque actual value Ch.1 | PDO Mapping TxPDO 4 | UINT8 | RO | 0x01 (1dec) |
| 1A03:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x08 (Torque actual value)) | UINT32 | RO | 0x6010:08, 16 |

##### Index 1A04 DRV TxPDO-Map Info data 1 Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A04:0 | DRV TxPDO-Map Info data 1 Ch.1 | PDO Mapping TxPDO 5 | UINT8 | RW | 0x01 (1dec) |
| 1A04:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x12 (Info data 1)) | UINT32 | RW | 0x6010:12, 16 |
| 1A04:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A04:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A05 DRV TxPDO-Map Info data 2 Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A05:0 | DRV TxPDO-Map Info data 2 Ch.1 | PDO Mapping TxPDO 6 | UINT8 | RW | 0x01 (1dec) |
| 1A05:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x13 (Info data 2)) | UINT32 | RW | 0x6010:13, 16 |
| 1A05:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A05:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A06 DRV TxPDO-Map Following error actual value Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A06:0 | DRV TxPDO-Map Following error actual value Ch.1 | PDO Mapping TxPDO 7 | UINT8 | RO | 0x01 (1dec) |
| 1A06:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x06 (Following error actual value)) | UINT32 | RO | 0x6010:06, 32 |

##### Index 1A07 FB TxPDO-Map Touch probe status Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A07:0 | FB TxPDO-Map Touch probe status Ch.1 | PDO Mapping TxPDO 8 | UINT8 | RO | 0x0A (10dec) |
| 1A07:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x01 (TP1 Enable)) | UINT32 | RO | 0x6001:01, 1 |
| 1A07:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x02 (TP1 Pos value stored)) | UINT32 | RO | 0x6001:02, 1 |
| 1A07:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x03 (TP1 Neg value stored)) | UINT32 | RO | 0x6001:03, 1 |
| 1A07:04 | SubIndex 004 | 4. PDO Mapping entry (4 bits align) | UINT32 | RO | 0x0000:00, 4 |
| 1A07:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x08 (TP1 Input)) | UINT32 | RO | 0x6001:08, 1 |
| 1A07:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x09 (TP2 Enable)) | UINT32 | RO | 0x6001:09, 1 |
| 1A07:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x0A (TP2 Pos value stored)) | UINT32 | RO | 0x6001:0A, 1 |
| 1A07:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x0B (TP2 Neg value stored)) | UINT32 | RO | 0x6001:0B, 1 |
| 1A07:09 | SubIndex 009 | 9. PDO Mapping entry (4 bits align) | UINT32 | RO | 0x0000:00, 4 |
| 1A07:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x10 (TP2 Input)) | UINT32 | RO | 0x6001:10, 1 |

##### Index 1A08 FB TxPDO-Map Touch probe 1 pos position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A08:0 | FB TxPDO-Map Touch probe 1 pos position Ch.1 | PDO Mapping TxPDO 9 | UINT8 | RO | 0x01 (1dec) |
| 1A08:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x11 (TP1 Pos position)) | UINT32 | RO | 0x6001:11, 32 |

##### Index 1A09 FB TxPDO-Map Touch probe 1 neg position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A09:0 | FB TxPDO-Map Touch probe 1 neg position Ch.1 | PDO Mapping TxPDO 10 | UINT8 | RO | 0x01 (1dec) |
| 1A09:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x12 (TP1 Neg position)) | UINT32 | RO | 0x6001:12, 32 |

##### Index 1A0A FB TxPDO-Map Touch probe 2 pos position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A0A:0 | FB TxPDO-Map Touch probe 2 pos position Ch.1 | PDO Mapping TxPDO 11 | UINT8 | RO | 0x01 (1dec) |
| 1A0A:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x13 (TP2 Pos position)) | UINT32 | RO | 0x6001:13, 32 |

##### Index 1A0B FB TxPDO-Map Touch probe 2 neg position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A0B:0 | FB TxPDO-Map Touch probe 2 neg position Ch.1 | PDO Mapping TxPDO 12 | UINT8 | RO | 0x01 (1dec) |
| 1A0B:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x14 (TP2 Neg position)) | UINT32 | RO | 0x6001:14, 32 |

##### Index 1A0D DRV TxPDO-Map Info data 3 Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A0D:0 | DRV TxPDO-Map Info data 3 Ch.1 | PDO Mapping TxPDO 14 | UINT8 | RW | 0x01 (1dec) |
| 1A0D:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x14 (Info data 3)) | UINT32 | RW | 0x6010:14, 16 |
| 1A0D:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A0D:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A0E DRV TxPDO-Map Modes of operation display Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A0E:0 | DRV TxPDO-Map Modes of operation display Ch.1 | PDO Mapping TxPDO 15 | UINT8 | RO | 0x01 (1dec) |
| 1A0E:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x03 (Modes of operation display)) | UINT32 | RO | 0x6010:03, 8 |

##### Index 1A0F DRV TxPDO-Map Torque limitation status Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A0F:0 | DRV TxPDO-Map Torque limitation status Ch.1 | PDO Mapping TxPDO 16 | UINT8 | RO | 0x01 (1dec) |
| 1A0F:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6010 (DRV Inputs Ch.1), entry 0x15 (Torque limitation status)) | UINT32 | RO | 0x6010:15, 8 |

##### Index 1A10 DI TxPDO-Map Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A10:0 | DI TxPDO-Map Inputs Ch.1 | PDO Mapping TxPDO 17 | UINT8 | RO | 0x07 (7dec) |
| 1A10:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6020 (DI Inputs Ch.1), entry 0x01 (Input 1)) | UINT32 | RO | 0x6020:01, 1 |
| 1A10:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6020 (DI Inputs Ch.1), entry 0x02 (Input 2)) | UINT32 | RO | 0x6020:02, 1 |
| 1A10:03 | SubIndex 003 | 3. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |
| 1A10:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x6020 (DI Inputs Ch.1), entry 0x05 (Encoder A)) | UINT32 | RO | 0x6020:05, 1 |
| 1A10:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6020 (DI Inputs Ch.1), entry 0x06 (Encoder B)) | UINT32 | RO | 0x6020:06, 1 |
| 1A10:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x6020 (DI Inputs Ch.1), entry 0x07 (Encoder C)) | UINT32 | RO | 0x6020:07, 1 |
| 1A10:07 | SubIndex 007 | 7. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |

##### Index 1A11 FB TxPDO-Map Touch probe 1 pos timestamp Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A11:0 | FB TxPDO-Map Touch probe 1 pos timestamp Ch.1 | PDO Mapping TxPDO 18 | UINT8 | RO | 0x01 (1dec) |
| 1A11:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x15 (TP1 Pos timestamp)) | UINT32 | RO | 0x6001:15, 32 |

##### Index 1A12 FB TxPDO-Map Touch probe 1 neg timestamp Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A12:0 | FB TxPDO-Map Touch probe 1 neg timestamp Ch.1 | PDO Mapping TxPDO 19 | UINT8 | RO | 0x01 (1dec) |
| 1A12:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x16 (TP1 Neg timestamp)) | UINT32 | RO | 0x6001:16, 32 |

##### Index 1A13 FB TxPDO-Map Touch probe 2 pos timestamp Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A13:0 | FB TxPDO-Map Touch probe 2 pos timestamp Ch.1 | PDO Mapping TxPDO 20 | UINT8 | RO | 0x01 (1dec) |
| 1A13:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x17 (TP2 Pos timestamp)) | UINT32 | RO | 0x6001:17, 32 |

##### Index 1A14 FB TxPDO-Map Touch probe 2 neg timestamp Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A14:0 | FB TxPDO-Map Touch probe 2 neg timestamp Ch.1 | PDO Mapping TxPDO 21 | UINT8 | RO | 0x01 (1dec) |
| 1A14:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6001 (FB Touch probe inputs Ch.1), entry 0x18 (TP2 Neg timestamp)) | UINT32 | RO | 0x6001:18, 32 |

##### Index 1A15 FB TxPDO-Map Secondary Position Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A15:0 | FB TxPDO-Map Secondary Position Ch.1 | PDO Mapping TxPDO 22 | UINT8 | RO | 0x01 (1dec) |
| 1A15:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6000 (FB Inputs Ch.1), entry 0x15 (Secondary position)) | UINT32 | RO | 0x6000:15, 32 |

##### Index 1A60 DMC TxPDO-Map Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A60:0 | DMC TxPDO-Map Inputs Ch.1 | PDO Mapping TxPDO 97 | UINT8 | RO | 0x26 (38dec) |
| 1A60:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1A60:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x02 (DMC__FeedbackStatus__Latch extern valid)) | UINT32 | RO | 0x6060:02, 1 |
| 1A60:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x03 (DMC__FeedbackStatus__Set counter done)) | UINT32 | RO | 0x6060:03, 1 |
| 1A60:04 | SubIndex 004 | 4. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |
| 1A60:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x0D (DMC__FeedbackStatus__Status of extern latch)) | UINT32 | RO | 0x6060:0D, 1 |
| 1A60:06 | SubIndex 006 | 6. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1A60:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x11 (DMC__DriveStatus__Ready to enable)) | UINT32 | RO | 0x6060:11, 1 |
| 1A60:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x12 (DMC__DriveStatus__Ready)) | UINT32 | RO | 0x6060:12, 1 |
| 1A60:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x13 (DMC__DriveStatus__Warning)) | UINT32 | RO | 0x6060:13, 1 |
| 1A60:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x14 (DMC__DriveStatus__Error)) | UINT32 | RO | 0x6060:14, 1 |
| 1A60:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x15 (DMC__DriveStatus__Moving positive)) | UINT32 | RO | 0x6060:15, 1 |
| 1A60:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x16 (DMC__DriveStatus__Moving negative)) | UINT32 | RO | 0x6060:16, 1 |
| 1A60:0D | SubIndex 013 | 13. PDO Mapping entry (5 bits align) | UINT32 | RO | 0x0000:00, 5 |
| 1A60:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x1C (DMC__DriveStatus__Digital input 1)) | UINT32 | RO | 0x6060:1C, 1 |
| 1A60:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x1D (DMC__DriveStatus__Digital input 2)) | UINT32 | RO | 0x6060:1D, 1 |
| 1A60:10 | SubIndex 016 | 16. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1A60:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x21 (DMC__PositioningStatus__Busy)) | UINT32 | RO | 0x6060:21, 1 |
| 1A60:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x22 (DMC__PositioningStatus__In-Target)) | UINT32 | RO | 0x6060:22, 1 |
| 1A60:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x23 (DMC__PositioningStatus__Warning)) | UINT32 | RO | 0x6060:23, 1 |
| 1A60:14 | SubIndex 020 | 20. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x24 (DMC__PositioningStatus__Error)) | UINT32 | RO | 0x6060:24, 1 |
| 1A60:15 | SubIndex 021 | 21. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x25 (DMC__PositioningStatus__Calibrated)) | UINT32 | RO | 0x6060:25, 1 |
| 1A60:16 | SubIndex 022 | 22. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x26 (DMC__PositioningStatus__Accelerate)) | UINT32 | RO | 0x6060:26, 1 |
| 1A60:17 | SubIndex 023 | 23. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x27 (DMC__PositioningStatus__Decelerate)) | UINT32 | RO | 0x6060:27, 1 |
| 1A60:18 | SubIndex 024 | 24. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x28 (DMC__PositioningStatus__Ready to execute)) | UINT32 | RO | 0x6060:28, 1 |
| 1A60:19 | SubIndex 025 | 25. PDO Mapping entry (8 bits align) | UINT32 | RO | 0x0000:00, 8 |
| 1A60:1A | SubIndex 026 | 26. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x31 (DMC__Set position)) | UINT32 | RO | 0x6060:31, 64 |
| 1A60:1B | SubIndex 027 | 27. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x32 (DMC__Set velocity)) | UINT32 | RO | 0x6060:32, 16 |
| 1A60:1C | SubIndex 028 | 28. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x33 (DMC__Actual drive time)) | UINT32 | RO | 0x6060:33, 32 |
| 1A60:1D | SubIndex 029 | 29. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x34 (DMC__Actual position lag)) | UINT32 | RO | 0x6060:34, 64 |
| 1A60:1E | SubIndex 030 | 30. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x35 (DMC__Actual velocity)) | UINT32 | RO | 0x6060:35, 16 |
| 1A60:1F | SubIndex 031 | 31. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x36 (DMC__Actual position)) | UINT32 | RO | 0x6060:36, 64 |
| 1A60:20 | SubIndex 032 | 32. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x37 (DMC__Error id)) | UINT32 | RO | 0x6060:37, 32 |
| 1A60:21 | SubIndex 033 | 33. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x38 (DMC__Input cycle counter)) | UINT32 | RO | 0x6060:38, 8 |
| 1A60:22 | SubIndex 034 | 34. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x39 (DMC__Channel id)) | UINT32 | RO | 0x6060:39, 8 |
| 1A60:23 | SubIndex 035 | 35. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3A (DMC__Latch value)) | UINT32 | RO | 0x6060:3A, 64 |
| 1A60:24 | SubIndex 036 | 36. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3B (DMC__Cyclic info data 1)) | UINT32 | RO | 0x6060:3B, 16 |
| 1A60:25 | SubIndex 037 | 37. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3C (DMC__Cyclic info data 2)) | UINT32 | RO | 0x6060:3C, 16 |
| 1A60:26 | SubIndex 038 | 38. PDO Mapping entry (64 bits align) | UINT32 | RO | 0x0000:00, 64 |

##### Index 1A61 DMC TxPDO-Map Inputs 32 Bit Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A61:0 | DMC TxPDO-Map Inputs 32 Bit Ch.1 | PDO Mapping TxPDO 98 | UINT8 | RO | 0x2A (42dec) |
| 1A61:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1A61:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x02 (DMC__FeedbackStatus__Latch extern valid)) | UINT32 | RO | 0x6060:02, 1 |
| 1A61:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x03 (DMC__FeedbackStatus__Set counter done)) | UINT32 | RO | 0x6060:03, 1 |
| 1A61:04 | SubIndex 004 | 4. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |
| 1A61:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x0D (DMC__FeedbackStatus__Status of extern latch)) | UINT32 | RO | 0x6060:0D, 1 |
| 1A61:06 | SubIndex 006 | 6. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1A61:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x11 (DMC__DriveStatus__Ready to enable)) | UINT32 | RO | 0x6060:11, 1 |
| 1A61:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x12 (DMC__DriveStatus__Ready)) | UINT32 | RO | 0x6060:12, 1 |
| 1A61:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x13 (DMC__DriveStatus__Warning)) | UINT32 | RO | 0x6060:13, 1 |
| 1A61:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x14 (DMC__DriveStatus__Error)) | UINT32 | RO | 0x6060:14, 1 |
| 1A61:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x15 (DMC__DriveStatus__Moving positive)) | UINT32 | RO | 0x6060:15, 1 |
| 1A61:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x16 (DMC__DriveStatus__Moving negative)) | UINT32 | RO | 0x6060:16, 1 |
| 1A61:0D | SubIndex 013 | 13. PDO Mapping entry (5 bits align) | UINT32 | RO | 0x0000:00, 5 |
| 1A61:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x1C (DMC__DriveStatus__Digital input 1)) | UINT32 | RO | 0x6060:1C, 1 |
| 1A61:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x1D (DMC__DriveStatus__Digital input 2)) | UINT32 | RO | 0x6060:1D, 1 |
| 1A61:10 | SubIndex 016 | 16. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1A61:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x21 (DMC__PositioningStatus__Busy)) | UINT32 | RO | 0x6060:21, 1 |
| 1A61:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x22 (DMC__PositioningStatus__In-Target)) | UINT32 | RO | 0x6060:22, 1 |
| 1A61:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x23 (DMC__PositioningStatus__Warning)) | UINT32 | RO | 0x6060:23, 1 |
| 1A61:14 | SubIndex 020 | 20. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x24 (DMC__PositioningStatus__Error)) | UINT32 | RO | 0x6060:24, 1 |
| 1A61:15 | SubIndex 021 | 21. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x25 (DMC__PositioningStatus__Calibrated)) | UINT32 | RO | 0x6060:25, 1 |
| 1A61:16 | SubIndex 022 | 22. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x26 (DMC__PositioningStatus__Accelerate)) | UINT32 | RO | 0x6060:26, 1 |
| 1A61:17 | SubIndex 023 | 23. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x27 (DMC__PositioningStatus__Decelerate)) | UINT32 | RO | 0x6060:27, 1 |
| 1A61:18 | SubIndex 024 | 24. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x28 (DMC__PositioningStatus__Ready to execute)) | UINT32 | RO | 0x6060:28, 1 |
| 1A61:19 | SubIndex 025 | 25. PDO Mapping entry (8 bits align) | UINT32 | RO | 0x0000:00, 8 |
| 1A61:1A | SubIndex 026 | 26. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x31 (DMC__Set position)) | UINT32 | RO | 0x6060:31, 32 |
| 1A61:1B | SubIndex 027 | 27. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1A61:1C | SubIndex 028 | 28. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x32 (DMC__Set velocity)) | UINT32 | RO | 0x6060:32, 16 |
| 1A61:1D | SubIndex 029 | 29. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x33 (DMC__Actual drive time)) | UINT32 | RO | 0x6060:33, 32 |
| 1A61:1E | SubIndex 030 | 30. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x34 (DMC__Actual position lag)) | UINT32 | RO | 0x6060:34, 32 |
| 1A61:1F | SubIndex 031 | 31. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1A61:20 | SubIndex 032 | 32. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x35 (DMC__Actual velocity)) | UINT32 | RO | 0x6060:35, 16 |
| 1A61:21 | SubIndex 033 | 33. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x36 (DMC__Actual position)) | UINT32 | RO | 0x6060:36, 32 |
| 1A61:22 | SubIndex 034 | 34. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1A61:23 | SubIndex 035 | 35. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x37 (DMC__Error id)) | UINT32 | RO | 0x6060:37, 32 |
| 1A61:24 | SubIndex 036 | 36. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x38 (DMC__Input cycle counter)) | UINT32 | RO | 0x6060:38, 8 |
| 1A61:25 | SubIndex 037 | 37. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x39 (DMC__Channel id)) | UINT32 | RO | 0x6060:39, 8 |
| 1A61:26 | SubIndex 038 | 38. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3A (DMC__Latch value)) | UINT32 | RO | 0x6060:3A, 32 |
| 1A61:27 | SubIndex 039 | 39. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1A61:28 | SubIndex 040 | 40. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3B (DMC__Cyclic info data 1)) | UINT32 | RO | 0x6060:3B, 16 |
| 1A61:29 | SubIndex 041 | 41. PDO Mapping entry (object 0x6060 (DMC Inputs Ch.1), entry 0x3C (DMC__Cyclic info data 2)) | UINT32 | RO | 0x6060:3C, 16 |
| 1A61:2A | SubIndex 042 | 42. PDO Mapping entry (64 bits align) | UINT32 | RO | 0x0000:00, 64 |

##### Index 1A80 FB TxPDO-Map Position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A80:0 | FB TxPDO-Map Position Ch.2 | PDO Mapping TxPDO 129 | UINT8 | RO | 0x01 (1dec) |
| 1A80:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6100 (FB Inputs Ch.2), entry 0x11 (Position)) | UINT32 | RO | 0x6100:11, 32 |

##### Index 1A81 DRV TxPDO-Map Statusword Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A81:0 | DRV TxPDO-Map Statusword Ch.2 | PDO Mapping TxPDO 130 | UINT8 | RO | 0x01 (1dec) |
| 1A81:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x01 (Statusword)) | UINT32 | RO | 0x6110:01, 16 |

##### Index 1A82 DRV TxPDO-Map Velocity actual value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A82:0 | DRV TxPDO-Map Velocity actual value Ch.2 | PDO Mapping TxPDO 131 | UINT8 | RO | 0x01 (1dec) |
| 1A82:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x07 (Velocity actual value)) | UINT32 | RO | 0x6110:07, 32 |

##### Index 1A83 DRV TxPDO-Map Torque actual value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A83:0 | DRV TxPDO-Map Torque actual value Ch.2 | PDO Mapping TxPDO 132 | UINT8 | RO | 0x01 (1dec) |
| 1A83:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x08 (Torque actual value)) | UINT32 | RO | 0x6110:08, 16 |

##### Index 1A84 DRV TxPDO-Map Info data 1 Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A84:0 | DRV TxPDO-Map Info data 1 Ch.2 | PDO Mapping TxPDO 133 | UINT8 | RW | 0x01 (1dec) |
| 1A84:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x12 (Info data 1)) | UINT32 | RW | 0x6110:12, 16 |
| 1A84:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A84:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A85 DRV TxPDO-Map Info data 2 Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A85:0 | DRV TxPDO-Map Info data 2 Ch.2 | PDO Mapping TxPDO 134 | UINT8 | RW | 0x01 (1dec) |
| 1A85:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x13 (Info data 2)) | UINT32 | RW | 0x6110:13, 16 |
| 1A85:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A85:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A86 DRV TxPDO-Map Following error actual value Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A86:0 | DRV TxPDO-Map Following error actual value Ch.2 | PDO Mapping TxPDO 135 | UINT8 | RO | 0x01 (1dec) |
| 1A86:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x06 (Following error actual value)) | UINT32 | RO | 0x6110:06, 32 |

##### Index 1A87 FB TxPDO-Map Touch probe status Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A87:0 | FB TxPDO-Map Touch probe status Ch.2 | PDO Mapping TxPDO 136 | UINT8 | RO | 0x0A (10dec) |
| 1A87:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x01 (TP1 Enable)) | UINT32 | RO | 0x6101:01, 1 |
| 1A87:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x02 (TP1 Pos value stored)) | UINT32 | RO | 0x6101:02, 1 |
| 1A87:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x03 (TP1 Neg value stored)) | UINT32 | RO | 0x6101:03, 1 |
| 1A87:04 | SubIndex 004 | 4. PDO Mapping entry (4 bits align) | UINT32 | RO | 0x0000:00, 4 |
| 1A87:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x08 (TP1 Input)) | UINT32 | RO | 0x6101:08, 1 |
| 1A87:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x09 (TP2 Enable)) | UINT32 | RO | 0x6101:09, 1 |
| 1A87:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x0A (TP2 Pos value stored)) | UINT32 | RO | 0x6101:0A, 1 |
| 1A87:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x0B (TP2 Neg value stored)) | UINT32 | RO | 0x6101:0B, 1 |
| 1A87:09 | SubIndex 009 | 9. PDO Mapping entry (4 bits align) | UINT32 | RO | 0x0000:00, 4 |
| 1A87:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x10 (TP2 Input)) | UINT32 | RO | 0x6101:10, 1 |

##### Index 1A88 FB TxPDO-Map Touch probe 1 pos position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A88:0 | FB TxPDO-Map Touch probe 1 pos position Ch.2 | PDO Mapping TxPDO 137 | UINT8 | RO | 0x01 (1dec) |
| 1A88:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x11 (TP1 Pos position)) | UINT32 | RO | 0x6101:11, 32 |

##### Index 1A89 FB TxPDO-Map Touch probe 1 neg position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A89:0 | FB TxPDO-Map Touch probe 1 neg position Ch.2 | PDO Mapping TxPDO 138 | UINT8 | RO | 0x01 (1dec) |
| 1A89:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x12 (TP1 Neg position)) | UINT32 | RO | 0x6101:12, 32 |

##### Index 1A8A FB TxPDO-Map Touch probe 2 pos position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A8A:0 | FB TxPDO-Map Touch probe 2 pos position Ch.2 | PDO Mapping TxPDO 139 | UINT8 | RO | 0x01 (1dec) |
| 1A8A:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x13 (TP2 Pos position)) | UINT32 | RO | 0x6101:13, 32 |

##### Index 1A8B FB TxPDO-Map Touch probe 2 neg position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A8B:0 | FB TxPDO-Map Touch probe 2 neg position Ch.2 | PDO Mapping TxPDO 140 | UINT8 | RO | 0x01 (1dec) |
| 1A8B:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x14 (TP2 Neg position)) | UINT32 | RO | 0x6101:14, 32 |

##### Index 1A8D DRV TxPDO-Map Info data 3 Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A8D:0 | DRV TxPDO-Map Info data 3 Ch.2 | PDO Mapping TxPDO 142 | UINT8 | RW | 0x01 (1dec) |
| 1A8D:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x14 (Info data 3)) | UINT32 | RW | 0x6110:14, 16 |
| 1A8D:02 | SubIndex 002 | 2. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:03 | SubIndex 003 | 3. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:04 | SubIndex 004 | 4. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:05 | SubIndex 005 | 5. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:06 | SubIndex 006 | 6. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:07 | SubIndex 007 | 7. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:08 | SubIndex 008 | 8. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:09 | SubIndex 009 | 9. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0A | SubIndex 010 | 10. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0B | SubIndex 011 | 11. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0C | SubIndex 012 | 12. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0D | SubIndex 013 | 13. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0E | SubIndex 014 | 14. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:0F | SubIndex 015 | 15. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |
| 1A8D:10 | SubIndex 016 | 16. PDO Mapping entry (0 bits align) | UINT32 | RW | 0x0000:00, 0 |

##### Index 1A8E DRV TxPDO-Map Modes of operation display Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A8E:0 | DRV TxPDO-Map Modes of operation display Ch.2 | PDO Mapping TxPDO 143 | UINT8 | RO | 0x01 (1dec) |
| 1A8E:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x03 (Modes of operation display)) | UINT32 | RO | 0x6110:03, 8 |

##### Index 1A8F DRV TxPDO-Map Torque limitation status Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A8F:0 | DRV TxPDO-Map Torque limitation status Ch.2 | PDO Mapping TxPDO 144 | UINT8 | RO | 0x01 (1dec) |
| 1A8F:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6110 (DRV Inputs Ch.2), entry 0x15 (Torque limitation status)) | UINT32 | RO | 0x6110:15, 8 |

##### Index 1A90 DI TxPDO-Map Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A90:0 | DI TxPDO-Map Inputs Ch.2 | PDO Mapping TxPDO 145 | UINT8 | RO | 0x07 (7dec) |
| 1A90:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6120 (DI Inputs Ch.2), entry 0x01 (Input 1)) | UINT32 | RO | 0x6120:01, 1 |
| 1A90:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6120 (DI Inputs Ch.2), entry 0x02 (Input 2)) | UINT32 | RO | 0x6120:02, 1 |
| 1A90:03 | SubIndex 003 | 3. PDO Mapping entry (2 bits align) | UINT32 | RO | 0x0000:00, 2 |
| 1A90:04 | SubIndex 004 | 4. PDO Mapping entry (object 0x6120 (DI Inputs Ch.2), entry 0x05 (Encoder A)) | UINT32 | RO | 0x6120:05, 1 |
| 1A90:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6120 (DI Inputs Ch.2), entry 0x06 (Encoder B)) | UINT32 | RO | 0x6120:06, 1 |
| 1A90:06 | SubIndex 006 | 6. PDO Mapping entry (object 0x6120 (DI Inputs Ch.2), entry 0x07 (Encoder C)) | UINT32 | RO | 0x6120:07, 1 |
| 1A90:07 | SubIndex 007 | 7. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |

##### Index 1A91 FB TxPDO-Map Touch probe 1 pos timestamp Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A91:0 | FB TxPDO-Map Touch probe 1 pos timestamp Ch.2 | PDO Mapping TxPDO 146 | UINT8 | RO | 0x01 (1dec) |
| 1A91:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x15 (TP1 Pos timestamp)) | UINT32 | RO | 0x6101:15, 32 |

##### Index 1A92 FB TxPDO-Map Touch probe 1 neg timestamp Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A92:0 | FB TxPDO-Map Touch probe 1 neg timestamp Ch.2 | PDO Mapping TxPDO 147 | UINT8 | RO | 0x01 (1dec) |
| 1A92:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x16 (TP1 Neg timestamp)) | UINT32 | RO | 0x6101:16, 32 |

##### Index 1A93 FB TxPDO-Map Touch probe 2 pos timestamp Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A93:0 | FB TxPDO-Map Touch probe 2 pos timestamp Ch.2 | PDO Mapping TxPDO 148 | UINT8 | RO | 0x01 (1dec) |
| 1A93:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x17 (TP2 Pos timestamp)) | UINT32 | RO | 0x6101:17, 32 |

##### Index 1A94 FB TxPDO-Map Touch probe 2 neg timestamp Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A94:0 | FB TxPDO-Map Touch probe 2 neg timestamp Ch.2 | PDO Mapping TxPDO 149 | UINT8 | RO | 0x01 (1dec) |
| 1A94:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6101 (FB Touch probe inputs Ch.2), entry 0x18 (TP2 Neg timestamp)) | UINT32 | RO | 0x6101:18, 32 |

##### Index 1A95 FB TxPDO-Map Secondary Position Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1A95:0 | FB TxPDO-Map Secondary Position Ch.2 | PDO Mapping TxPDO 150 | UINT8 | RO | 0x01 (1dec) |
| 1A95:01 | SubIndex 001 | 1. PDO Mapping entry (object 0x6100 (FB Inputs Ch.2), entry 0x15 (Secondary position)) | UINT32 | RO | 0x6100:15, 32 |

##### Index 1AE0 DMC TxPDO-Map Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1AE0:0 | DMC TxPDO-Map Inputs Ch.2 | PDO Mapping TxPDO 225 | UINT8 | RO | 0x26 (38dec) |
| 1AE0:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1AE0:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x02 (DMC__FeedbackStatus__Latch extern valid)) | UINT32 | RO | 0x6160:02, 1 |
| 1AE0:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x03 (DMC__FeedbackStatus__Set counter done)) | UINT32 | RO | 0x6160:03, 1 |
| 1AE0:04 | SubIndex 004 | 4. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |
| 1AE0:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x0D (DMC__FeedbackStatus__Status of extern latch)) | UINT32 | RO | 0x6160:0D, 1 |
| 1AE0:06 | SubIndex 006 | 6. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1AE0:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x11 (DMC__DriveStatus__Ready to enable)) | UINT32 | RO | 0x6160:11, 1 |
| 1AE0:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x12 (DMC__DriveStatus__Ready)) | UINT32 | RO | 0x6160:12, 1 |
| 1AE0:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x13 (DMC__DriveStatus__Warning)) | UINT32 | RO | 0x6160:13, 1 |
| 1AE0:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x14 (DMC__DriveStatus__Error)) | UINT32 | RO | 0x6160:14, 1 |
| 1AE0:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x15 (DMC__DriveStatus__Moving positive)) | UINT32 | RO | 0x6160:15, 1 |
| 1AE0:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x16 (DMC__DriveStatus__Moving negative)) | UINT32 | RO | 0x6160:16, 1 |
| 1AE0:0D | SubIndex 013 | 13. PDO Mapping entry (5 bits align) | UINT32 | RO | 0x0000:00, 5 |
| 1AE0:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x1C (DMC__DriveStatus__Digital input 1)) | UINT32 | RO | 0x6160:1C, 1 |
| 1AE0:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x1D (DMC__DriveStatus__Digital input 2)) | UINT32 | RO | 0x6160:1D, 1 |
| 1AE0:10 | SubIndex 016 | 16. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1AE0:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x21 (DMC__PositioningStatus__Busy)) | UINT32 | RO | 0x6160:21, 1 |
| 1AE0:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x22 (DMC__PositioningStatus__In-Target)) | UINT32 | RO | 0x6160:22, 1 |
| 1AE0:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x23 (DMC__PositioningStatus__Warning)) | UINT32 | RO | 0x6160:23, 1 |
| 1AE0:14 | SubIndex 020 | 20. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x24 (DMC__PositioningStatus__Error)) | UINT32 | RO | 0x6160:24, 1 |
| 1AE0:15 | SubIndex 021 | 21. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x25 (DMC__PositioningStatus__Calibrated)) | UINT32 | RO | 0x6160:25, 1 |
| 1AE0:16 | SubIndex 022 | 22. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x26 (DMC__PositioningStatus__Accelerate)) | UINT32 | RO | 0x6160:26, 1 |
| 1AE0:17 | SubIndex 023 | 23. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x27 (DMC__PositioningStatus__Decelerate)) | UINT32 | RO | 0x6160:27, 1 |
| 1AE0:18 | SubIndex 024 | 24. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x28 (DMC__PositioningStatus__Ready to execute)) | UINT32 | RO | 0x6160:28, 1 |
| 1AE0:19 | SubIndex 025 | 25. PDO Mapping entry (8 bits align) | UINT32 | RO | 0x0000:00, 8 |
| 1AE0:1A | SubIndex 026 | 26. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x31 (DMC__Set position)) | UINT32 | RO | 0x6160:31, 64 |
| 1AE0:1B | SubIndex 027 | 27. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x32 (DMC__Set velocity)) | UINT32 | RO | 0x6160:32, 16 |
| 1AE0:1C | SubIndex 028 | 28. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x33 (DMC__Actual drive time)) | UINT32 | RO | 0x6160:33, 32 |
| 1AE0:1D | SubIndex 029 | 29. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x34 (DMC__Actual position lag)) | UINT32 | RO | 0x6160:34, 64 |
| 1AE0:1E | SubIndex 030 | 30. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x35 (DMC__Actual velocity)) | UINT32 | RO | 0x6160:35, 16 |
| 1AE0:1F | SubIndex 031 | 31. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x36 (DMC__Actual position)) | UINT32 | RO | 0x6160:36, 64 |
| 1AE0:20 | SubIndex 032 | 32. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x37 (DMC__Error id)) | UINT32 | RO | 0x6160:37, 32 |
| 1AE0:21 | SubIndex 033 | 33. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x38 (DMC__Input cycle counter)) | UINT32 | RO | 0x6160:38, 8 |
| 1AE0:22 | SubIndex 034 | 34. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x39 (DMC__Channel id)) | UINT32 | RO | 0x6160:39, 8 |
| 1AE0:23 | SubIndex 035 | 35. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3A (DMC__Latch value)) | UINT32 | RO | 0x6160:3A, 64 |
| 1AE0:24 | SubIndex 036 | 36. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3B (DMC__Cyclic info data 1)) | UINT32 | RO | 0x6160:3B, 16 |
| 1AE0:25 | SubIndex 037 | 37. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3C (DMC__Cyclic info data 2)) | UINT32 | RO | 0x6160:3C, 16 |
| 1AE0:26 | SubIndex 038 | 38. PDO Mapping entry (64 bits align) | UINT32 | RO | 0x0000:00, 64 |

##### Index 1AE1 DMC TxPDO-Map Inputs 32 Bit Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1AE1:0 | DMC TxPDO-Map Inputs 32 Bit Ch.2 | PDO Mapping TxPDO 226 | UINT8 | RO | 0x2A (42dec) |
| 1AE1:01 | SubIndex 001 | 1. PDO Mapping entry (1 bits align) | UINT32 | RO | 0x0000:00, 1 |
| 1AE1:02 | SubIndex 002 | 2. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x02 (DMC__FeedbackStatus__Latch extern valid)) | UINT32 | RO | 0x6160:02, 1 |
| 1AE1:03 | SubIndex 003 | 3. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x03 (DMC__FeedbackStatus__Set counter done)) | UINT32 | RO | 0x6160:03, 1 |
| 1AE1:04 | SubIndex 004 | 4. PDO Mapping entry (9 bits align) | UINT32 | RO | 0x0000:00, 9 |
| 1AE1:05 | SubIndex 005 | 5. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x0D (DMC__FeedbackStatus__Status of extern latch)) | UINT32 | RO | 0x6160:0D, 1 |
| 1AE1:06 | SubIndex 006 | 6. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1AE1:07 | SubIndex 007 | 7. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x11 (DMC__DriveStatus__Ready to enable)) | UINT32 | RO | 0x6160:11, 1 |
| 1AE1:08 | SubIndex 008 | 8. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x12 (DMC__DriveStatus__Ready)) | UINT32 | RO | 0x6160:12, 1 |
| 1AE1:09 | SubIndex 009 | 9. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x13 (DMC__DriveStatus__Warning)) | UINT32 | RO | 0x6160:13, 1 |
| 1AE1:0A | SubIndex 010 | 10. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x14 (DMC__DriveStatus__Error)) | UINT32 | RO | 0x6160:14, 1 |
| 1AE1:0B | SubIndex 011 | 11. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x15 (DMC__DriveStatus__Moving positive)) | UINT32 | RO | 0x6160:15, 1 |
| 1AE1:0C | SubIndex 012 | 12. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x16 (DMC__DriveStatus__Moving negative)) | UINT32 | RO | 0x6160:16, 1 |
| 1AE1:0D | SubIndex 013 | 13. PDO Mapping entry (5 bits align) | UINT32 | RO | 0x0000:00, 5 |
| 1AE1:0E | SubIndex 014 | 14. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x1C (DMC__DriveStatus__Digital input 1)) | UINT32 | RO | 0x6160:1C, 1 |
| 1AE1:0F | SubIndex 015 | 15. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x1D (DMC__DriveStatus__Digital input 2)) | UINT32 | RO | 0x6160:1D, 1 |
| 1AE1:10 | SubIndex 016 | 16. PDO Mapping entry (3 bits align) | UINT32 | RO | 0x0000:00, 3 |
| 1AE1:11 | SubIndex 017 | 17. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x21 (DMC__PositioningStatus__Busy)) | UINT32 | RO | 0x6160:21, 1 |
| 1AE1:12 | SubIndex 018 | 18. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x22 (DMC__PositioningStatus__In-Target)) | UINT32 | RO | 0x6160:22, 1 |
| 1AE1:13 | SubIndex 019 | 19. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x23 (DMC__PositioningStatus__Warning)) | UINT32 | RO | 0x6160:23, 1 |
| 1AE1:14 | SubIndex 020 | 20. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x24 (DMC__PositioningStatus__Error)) | UINT32 | RO | 0x6160:24, 1 |
| 1AE1:15 | SubIndex 021 | 21. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x25 (DMC__PositioningStatus__Calibrated)) | UINT32 | RO | 0x6160:25, 1 |
| 1AE1:16 | SubIndex 022 | 22. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x26 (DMC__PositioningStatus__Accelerate)) | UINT32 | RO | 0x6160:26, 1 |
| 1AE1:17 | SubIndex 023 | 23. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x27 (DMC__PositioningStatus__Decelerate)) | UINT32 | RO | 0x6160:27, 1 |
| 1AE1:18 | SubIndex 024 | 24. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x28 (DMC__PositioningStatus__Ready to execute)) | UINT32 | RO | 0x6160:28, 1 |
| 1AE1:19 | SubIndex 025 | 25. PDO Mapping entry (8 bits align) | UINT32 | RO | 0x0000:00, 8 |
| 1AE1:1A | SubIndex 026 | 26. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x31 (DMC__Set position)) | UINT32 | RO | 0x6160:31, 32 |
| 1AE1:1B | SubIndex 027 | 27. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1AE1:1C | SubIndex 028 | 28. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x32 (DMC__Set velocity)) | UINT32 | RO | 0x6160:32, 16 |
| 1AE1:1D | SubIndex 029 | 29. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x33 (DMC__Actual drive time)) | UINT32 | RO | 0x6160:33, 32 |
| 1AE1:1E | SubIndex 030 | 30. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x34 (DMC__Actual position lag)) | UINT32 | RO | 0x6160:34, 32 |
| 1AE1:1F | SubIndex 031 | 31. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1AE1:20 | SubIndex 032 | 32. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x35 (DMC__Actual velocity)) | UINT32 | RO | 0x6160:35, 16 |
| 1AE1:21 | SubIndex 033 | 33. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x36 (DMC__Actual position)) | UINT32 | RO | 0x6160:36, 32 |
| 1AE1:22 | SubIndex 034 | 34. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1AE1:23 | SubIndex 035 | 35. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x37 (DMC__Error id)) | UINT32 | RO | 0x6160:37, 32 |
| 1AE1:24 | SubIndex 036 | 36. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x38 (DMC__Input cycle counter)) | UINT32 | RO | 0x6160:38, 8 |
| 1AE1:25 | SubIndex 037 | 37. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x39 (DMC__Channel id)) | UINT32 | RO | 0x6160:39, 8 |
| 1AE1:26 | SubIndex 038 | 38. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3A (DMC__Latch value)) | UINT32 | RO | 0x6160:3A, 32 |
| 1AE1:27 | SubIndex 039 | 39. PDO Mapping entry (32 bits align) | UINT32 | RO | 0x0000:00, 32 |
| 1AE1:28 | SubIndex 040 | 40. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3B (DMC__Cyclic info data 1)) | UINT32 | RO | 0x6160:3B, 16 |
| 1AE1:29 | SubIndex 041 | 41. PDO Mapping entry (object 0x6160 (DMC Inputs Ch.2), entry 0x3C (DMC__Cyclic info data 2)) | UINT32 | RO | 0x6160:3C, 16 |
| 1AE1:2A | SubIndex 042 | 42. PDO Mapping entry (64 bits align) | UINT32 | RO | 0x0000:00, 64 |

##### Index 1C00 Sync manager type

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1C00:0 | Sync manager type | Using the Sync Managers | UINT8 | RO | 0x04 (4dec) |
| 1C00:01 | SubIndex 001 | Sync-Manager Type Channel 1: Mailbox Write | UINT8 | RO | 0x01 (1dec) |
| 1C00:02 | SubIndex 002 | Sync-Manager Type Channel 2: Mailbox Read | UINT8 | RO | 0x02 (2dec) |
| 1C00:03 | SubIndex 003 | Sync-Manager Type Channel 3: Process Data Write (Outputs) | UINT8 | RO | 0x03 (3dec) |
| 1C00:04 | SubIndex 004 | Sync-Manager Type Channel 4: Process Data Read (Inputs) | UINT8 | RO | 0x04 (4dec) |

##### Index 1C12 RxPDO assign

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1C12:0 | RxPDO assign | PDO Assign Outputs | UINT8 | RW | 0x04 (4dec) |
| 1C12:01 | SubIndex 001 | 1. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x1600 (5632dec) |
| 1C12:02 | SubIndex 002 | 2. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x1606 (5638dec) |
| 1C12:03 | SubIndex 003 | 3. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x1680 (5760dec) |
| 1C12:04 | SubIndex 004 | 4. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x1686 (5766dec) |
| 1C12:05 | SubIndex 005 | 5. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:06 | SubIndex 006 | 6. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:07 | SubIndex 007 | 7. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:08 | SubIndex 008 | 8. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:09 | SubIndex 009 | 9. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0A | SubIndex 010 | 10. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0B | SubIndex 011 | 11. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0C | SubIndex 012 | 12. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0D | SubIndex 013 | 13. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0E | SubIndex 014 | 14. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:0F | SubIndex 015 | 15. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:10 | SubIndex 016 | 16. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:11 | SubIndex 017 | 17. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:12 | SubIndex 018 | 18. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:13 | SubIndex 019 | 19. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:14 | SubIndex 020 | 20. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:15 | SubIndex 021 | 21. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:16 | SubIndex 022 | 22. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:17 | SubIndex 023 | 23. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:18 | SubIndex 024 | 24. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:19 | SubIndex 025 | 25. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:1A | SubIndex 026 | 26. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:1B | SubIndex 027 | 27. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:1C | SubIndex 028 | 28. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:1D | SubIndex 029 | 29. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C12:1E | SubIndex 030 | 30. allocated RxPDO (contains the index of the associated RxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |

##### Index 1C13 TxPDO assign

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1C13:0 | TxPDO assign | PDO Assign Inputs | UINT8 | RW | 0x06 (6dec) |
| 1C13:01 | SubIndex 001 | 1. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A00 (6656dec) |
| 1C13:02 | SubIndex 002 | 2. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A01 (6657dec) |
| 1C13:03 | SubIndex 003 | 3. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A06 (6662dec) |
| 1C13:04 | SubIndex 004 | 4. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A80 (6784dec) |
| 1C13:05 | SubIndex 005 | 5. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A81 (6785dec) |
| 1C13:06 | SubIndex 006 | 6. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x1A86 (6790dec) |
| 1C13:07 | SubIndex 007 | 7. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:08 | SubIndex 008 | 8. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:09 | SubIndex 009 | 9. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0A | SubIndex 010 | 10. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0B | SubIndex 011 | 11. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0C | SubIndex 012 | 12. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0D | SubIndex 013 | 13. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0E | SubIndex 014 | 14. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:0F | SubIndex 015 | 15. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:10 | SubIndex 016 | 16. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:11 | SubIndex 017 | 17. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:12 | SubIndex 018 | 18. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:13 | SubIndex 019 | 19. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:14 | SubIndex 020 | 20. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:15 | SubIndex 021 | 21. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:16 | SubIndex 022 | 22. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:17 | SubIndex 023 | 23. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:18 | SubIndex 024 | 24. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:19 | SubIndex 025 | 25. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1A | SubIndex 026 | 26. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1B | SubIndex 027 | 27. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1C | SubIndex 028 | 28. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1D | SubIndex 029 | 29. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1E | SubIndex 030 | 30. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:1F | SubIndex 031 | 31. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:20 | SubIndex 032 | 32. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:21 | SubIndex 033 | 33. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:22 | SubIndex 034 | 34. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:23 | SubIndex 035 | 35. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:24 | SubIndex 036 | 36. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:25 | SubIndex 037 | 37. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:26 | SubIndex 038 | 38. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:27 | SubIndex 039 | 39. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:28 | SubIndex 040 | 40. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:29 | SubIndex 041 | 41. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:2A | SubIndex 042 | 42. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:2B | SubIndex 043 | 43. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |
| 1C13:2C | SubIndex 044 | 44. allocated TxPDO (contains the index of the associated TxPDO mapping object) | UINT16 | RW | 0x0000 (0dec) |

##### Index 1C32 SM output parameter

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1C32:0 | SM output parameter | Synchronization parameters for the outputs | UINT8 | RO | 0x20 (32dec) |
| 1C32:01 | Sync mode | Current synchronization mode:<br>• 0: Free Run<br>• 1: Synchron with SM 2 Event<br>• 2: DC-Mode - Synchron with SYNC0 Event<br>• 3: DC-Mode - Synchron with SYNC1 Event | UINT16 | RW | 0x0003 (3dec) |
| 1C32:02 | Cycle time | Cycle time (in ns):<br>• Free Run: cycle time of the local timer<br>• Synchron with SM 2 Event: cycle time of the master<br>• DC-Mode: SYNC0/SYNC1 Cycle Time | UINT32 | RW | 0x000F4240 (1000000dec) |
| 1C32:03 | Shift time | Time between SYNC0 event and output of the outputs (in ns, DC Mode only) | UINT32 | RO | 0x00000000 (0dec) |
| 1C32:04 | Sync modes supported | Supported synchronization modes:<br>• Bit 0 = 1: Free Run is supported<br>• Bit 1 = 1: Synchron with SM 2 Event is supported<br>• Bit 2-3 = 01: DC-Mode is supported<br>• Bit 4-5 = 10: Output Shift with SYNC1 Event (only DC mode)<br>• Bit 14 = 1: dynamic times (measurement through writing of 1C32:08) | UINT16 | RO | 0x0812 (2066dec) |
| 1C32:05 | Minimum cycle time | Minimum cycle time (in ns) | UINT32 | RO | 0x0001E848 (125000dec) |
| 1C32:06 | Calc and copy time | Minimum time between SYNC0 and SYNC1 event (in ns, DC Mode only) | UINT32 | RO | 0x00007530 (30000dec) |
| 1C32:07 | Minimum delay time | | UINT32 | RO | 0x00007A12 (31250dec) |
| 1C32:08 | Get Cycle Time | • 0: Measurement of the local cycle time is stopped<br>• 1: Measurement of the local cycle time is started | UINT16 | RW | 0x0000 (0dec) |
| 1C32:09 | Maximum delay time | Time between SYNC1 event and output of the outputs (in ns, DC Mode only) | UINT32 | RO | 0x00007A12 (31250dec) |
| 1C32:0A | Sync0 Cycle Time | | UINT32 | RO | 0x0000F424 (62500dec) |
| 1C32:0B | SM event missed counter | Number of missed SM events in OPERATIONAL (DC Mode only) | UINT16 | RO | 0x0000 (0dec) |
| 1C32:0C | Cycle exceeded counter | Number of occasions the cycle time was exceeded in OPERATIONAL (cycle was not completed in time or the next cycle began too early) | UINT16 | RO | 0x0000 (0dec) |
| 1C32:0D | Shift too short counter | Number of intervals between SYNC0 and SYNC1 events that are too short (DC Mode only) | UINT16 | RO | 0x0000 (0dec) |
| 1C32:20 | Sync error | The synchronization was not correct in the last cycle (outputs were output too late; DC Mode only) | BOOLEAN | RO | 0x00 (0dec) |

##### Index 1C33 SM input parameter

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1C33:0 | SM input parameter | Synchronization parameters for the inputs | UINT8 | RO | 0x20 (32dec) |
| 1C33:01 | Sync mode | Current synchronization mode:<br>• 0: Free Run<br>• 1: Synchron with SM 3 Event (no outputs available)<br>• 2: DC - Synchron with SYNC0 Event<br>• 3: DC - Synchron with SYNC1 Event<br>• 34: Synchron with SM 2 Event (outputs available) | UINT16 | RW | 0x0003 (3dec) |
| 1C33:02 | Cycle time | as 1C32:02 | UINT32 | RW | 0x000F4240 (1000000dec) |
| 1C33:03 | Shift time | Time between SYNC0 event and reading of the inputs (in ns, DC Mode only) | UINT32 | RO | 0x00000000 (0dec) |
| 1C33:04 | Sync modes supported | Supported synchronization modes:<br>• Bit 0: Free Run is supported<br>• Bit 1: Synchron with SM 2 Event is supported (outputs available)<br>• Bit 1: Synchron with SM 3 Event is supported (no outputs available)<br>• Bit 2-3 = 01: DC-Mode is supported<br>• Bit 4-5 = 01: Input shift through local event (outputs available)<br>• Bit 4-5 = 10: Input shift with SYNC1 event (no outputs available)<br>• Bit 14 = 1: dynamic times (measurement through writing of 1C32:08 or 1C33:08) | UINT16 | RO | 0x0012 (18dec) |
| 1C33:05 | Minimum cycle time | as 1C32:05 | UINT32 | RO | 0x0001E848 (125000dec) |
| 1C33:06 | Calc and copy time | Time between reading of the inputs and the inputs being available for the master (in ns, DC Mode only) | UINT32 | RO | 0x00007530 (30000dec) |
| 1C33:07 | Minimum delay time | | UINT32 | RO | 0x00007A12 (31250dec) |
| 1C33:08 | Get Cycle Time | as 1C32:08 | UINT16 | RW | 0x0000 (0dec) |
| 1C33:09 | Maximum delay time | Time between SYNC1 event and reading of the inputs (in ns, DC Mode only) | UINT32 | RO | 0x00007A12 (31250dec) |
| 1C33:0A | Sync0 Cycle Time | | UINT32 | RO | 0x0000F424 (62500dec) |
| 1C33:0B | SM event missed counter | as 1C32:11 | UINT16 | RO | 0x0000 (0dec) |
| 1C33:0C | Cycle exceeded counter | as 1C32:12 | UINT16 | RO | 0x0000 (0dec) |
| 1C33:0D | Shift too short counter | as 1C32:13 | UINT16 | RO | 0x0000 (0dec) |
| 1C33:20 | Sync error | as 1C32:32 | BOOLEAN | RO | 0x00 (0dec) |

#### 8.1.4 Profile-specific objects

##### Index 6000 FB Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6000:0 | FB Inputs Ch.1 | | UINT8 | RO | 0x15 (21dec) |
| 6000:0E | TxPDO State | | BOOLEAN | RO | 0x00 (0dec) |
| 6000:0F | Input cycle counter | | BIT2 | RO | 0x00 (0dec) |
| 6000:11 | Position | | UINT32 | RO | 0x00000000 (0dec) |
| 6000:15 | Secondary position | | UINT32 | RO | 0x00000000 (0dec) |

##### Index 6001 FB Touch probe inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6001:0 | FB Touch probe inputs Ch.1 | | UINT8 | RO | 0x18 (24dec) |
| 6001:01 | TP1 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:02 | TP1 Pos value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:03 | TP1 Neg value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:08 | TP1 Input | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:09 | TP2 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:0A | TP2 Pos value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:0B | TP2 Neg value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:10 | TP2 Input | | BOOLEAN | RO | 0x00 (0dec) |
| 6001:11 | TP1 Pos position | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:12 | TP1 Neg position | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:13 | TP2 Pos position | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:14 | TP2 Neg position | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:15 | TP1 Pos timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:16 | TP1 Neg timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:17 | TP2 Pos timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6001:18 | TP2 Neg timestamp | | UINT32 | RO | 0x00000000 (0dec) |

##### Index 6010 DRV Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6010:0 | DRV Inputs Ch.1 | | UINT8 | RO | 0x15 (21dec) |
| 6010:01 | Statusword | Bit 0 : Ready to switch on<br>Bit 1 : Switched on<br>Bit 2 : Operation enabled<br>Bit 3 : Fault<br>Bit 4 : reserved<br>Bit 5 : reserved<br>Bit 6 : Switch on disabled<br>Bit 7 : Warning<br>Bit 8 + 9 : reserved<br>Bit 10 : TxPDOToggle<br>Bit 11 : Internal limit active<br>Bit 12 : Drive follows the command value<br>Bit 13 : Input cycle counter<br>Bit 14 - 15 : reserved | UINT16 | RO | 0x0000 (0dec) |
| 6010:03 | Modes of operation display | permitted values:<br>• 8: Cyclic synchronous position mode (CSP)<br>• 9: Cyclic synchronous velocity mode (CSV)<br>• 10: Cyclic synchronous torque mode (CST)<br>• 11: Cyclic synchronous torque mode with commutation angle (CSTCA)<br>• 131: Drive Motion Control (DMC) | UINT8 | RO | 0x00 (0dec) |
| 6010:06 | Following error actual value | | INT32 | RO | 0x00000000 (0dec) |
| 6010:07 | Velocity actual value | | INT32 | RO | 0x00000000 (0dec) |
| 6010:08 | Torque actual value | | INT16 | RO | 0x0000 (0dec) |
| 6010:12 | Info data 1 | | UINT16 | RO | 0x0000 (0dec) |
| 6010:13 | Info data 2 | | UINT16 | RO | 0x0000 (0dec) |
| 6010:14 | Info data 3 | | UINT16 | RO | 0x0000 (0dec) |
| 6010:15 | Torque limitation status | Bit 0 : Torque demand value is equal to ramp input<br>Bit 1 : High velocity limit active<br>Bit 2 : Low velocity limit active<br>Bit 3 - 7 : reserved | UINT8 | RO | 0x00 (0dec) |

##### Index 6020 DI Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6020:0 | DI Inputs Ch.1 | | UINT8 | RO | 0x07 (7dec) |
| 6020:01 | Input 1 | | BOOLEAN | RO | 0x00 (0dec) |
| 6020:02 | Input 2 | | BOOLEAN | RO | 0x00 (0dec) |
| 6020:05 | Encoder A | | BOOLEAN | RO | 0x00 (0dec) |
| 6020:06 | Encoder B | | BOOLEAN | RO | 0x00 (0dec) |
| 6020:07 | Encoder C | | BOOLEAN | RO | 0x00 (0dec) |

##### Index 6060 DMC Inputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6060:0 | DMC Inputs Ch.1 | | UINT8 | RO | 0x3C (60dec) |
| 6060:02 | DMC__FeedbackStatus__Latch extern valid | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:03 | DMC__FeedbackStatus__Set counter done | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:0D | DMC__FeedbackStatus__Status of extern latch | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:11 | DMC__DriveStatus__Ready to enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:12 | DMC__DriveStatus__Ready | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:13 | DMC__DriveStatus__Warning | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:14 | DMC__DriveStatus__Error | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:15 | DMC__DriveStatus__Moving positive | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:16 | DMC__DriveStatus__Moving negative | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:1C | DMC__DriveStatus__Digital input 1 | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:1D | DMC__DriveStatus__Digital input 2 | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:21 | DMC__PositioningStatus__Busy | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:22 | DMC__PositioningStatus__In-Target | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:23 | DMC__PositioningStatus__Warning | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:24 | DMC__PositioningStatus__Error | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:25 | DMC__PositioningStatus__Calibrated | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:26 | DMC__PositioningStatus__Accelerate | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:27 | DMC__PositioningStatus__Decelerate | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:28 | DMC__PositioningStatus__Ready to execute | | BOOLEAN | RO | 0x00 (0dec) |
| 6060:31 | DMC__Set position | | INT64 | RO | |
| 6060:32 | DMC__Set velocity | | INT16 | RO | 0x0000 (0dec) |
| 6060:33 | DMC__Actual drive time | | UINT32 | RO | 0x00000000 (0dec) |
| 6060:34 | DMC__Actual position lag | | INT64 | RO | |
| 6060:35 | DMC__Actual velocity | | INT16 | RO | 0x0000 (0dec) |
| 6060:36 | DMC__Actual position | | INT64 | RO | |
| 6060:37 | DMC__Error id | | UINT32 | RO | 0x00000000 (0dec) |
| 6060:38 | DMC__Input cycle counter | | UINT8 | RO | 0x00 (0dec) |
| 6060:39 | DMC__Channel id | | UINT8 | RO | 0x00 (0dec) |
| 6060:3A | DMC__Latch value | | INT64 | RO | |
| 6060:3B | DMC__Cyclic info data 1 | | INT16 | RO | 0x0000 (0dec) |
| 6060:3C | DMC__Cyclic info data 2 | | INT16 | RO | 0x0000 (0dec) |

##### Index 6100 FB Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6100:0 | FB Inputs Ch.2 | | UINT8 | RO | 0x15 (21dec) |
| 6100:0E | TxPDO State | | BOOLEAN | RO | 0x00 (0dec) |
| 6100:0F | Input cycle counter | | BIT2 | RO | 0x00 (0dec) |
| 6100:11 | Position | | UINT32 | RO | 0x00000000 (0dec) |
| 6100:15 | Secondary position | | UINT32 | RO | 0x00000000 (0dec) |

##### Index 6101 FB Touch probe inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6101:0 | FB Touch probe inputs Ch.2 | | UINT8 | RO | 0x18 (24dec) |
| 6101:01 | TP1 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:02 | TP1 Pos value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:03 | TP1 Neg value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:08 | TP1 Input | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:09 | TP2 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:0A | TP2 Pos value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:0B | TP2 Neg value stored | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:10 | TP2 Input | | BOOLEAN | RO | 0x00 (0dec) |
| 6101:11 | TP1 Pos position | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:12 | TP1 Neg position | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:13 | TP2 Pos position | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:14 | TP2 Neg position | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:15 | TP1 Pos timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:16 | TP1 Neg timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:17 | TP2 Pos timestamp | | UINT32 | RO | 0x00000000 (0dec) |
| 6101:18 | TP2 Neg timestamp | | UINT32 | RO | 0x00000000 (0dec) |

##### Index 6110 DRV Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6110:0 | DRV Inputs Ch.2 | | UINT8 | RO | 0x15 (21dec) |
| 6110:01 | Statusword | Bit 0 : Ready to switch on<br>Bit 1 : Switched on<br>Bit 2 : Operation enabled<br>Bit 3 : Fault<br>Bit 4 : reserved<br>Bit 5 : reserved<br>Bit 6 : Switch on disabled<br>Bit 7 : Warning<br>Bit 8 + 9 : reserved<br>Bit 10 : TxPDOToggle<br>Bit 11 : Internal limit active<br>Bit 12 : Drive follows the command value<br>Bit 13 : Input cycle counter<br>Bit 14 - 15 : reserved | UINT16 | RO | 0x0000 (0dec) |
| 6110:03 | Modes of operation display | permitted values:<br>• 8: Cyclic synchronous position mode (CSP)<br>• 9: Cyclic synchronous velocity mode (CSV)<br>• 10: Cyclic synchronous torque mode (CST)<br>• 11: Cyclic synchronous torque mode with commutation angle (CSTCA)<br>• 131: Drive Motion Control (DMC) | UINT8 | RO | 0x00 (0dec) |
| 6110:06 | Following error actual value | | INT32 | RO | 0x00000000 (0dec) |
| 6110:07 | Velocity actual value | | INT32 | RO | 0x00000000 (0dec) |
| 6110:08 | Torque actual value | | INT16 | RO | 0x0000 (0dec) |
| 6110:12 | Info data 1 | | UINT16 | RO | 0x0000 (0dec) |
| 6110:13 | Info data 2 | | UINT16 | RO | 0x0000 (0dec) |
| 6110:14 | Info data 3 | | UINT16 | RO | 0x0000 (0dec) |
| 6110:15 | Torque limitation status | Bit 0 : Torque demand value is equal to ramp input<br>Bit 1 : High velocity limit active<br>Bit 2 : Low velocity limit active<br>Bit 3 - 7 : reserved | UINT8 | RO | 0x00 (0dec) |

##### Index 6120 DI Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6120:0 | DI Inputs Ch.2 | | UINT8 | RO | 0x07 (7dec) |
| 6120:01 | Input 1 | | BOOLEAN | RO | 0x00 (0dec) |
| 6120:02 | Input 2 | | BOOLEAN | RO | 0x00 (0dec) |
| 6120:05 | Encoder A | | BOOLEAN | RO | 0x00 (0dec) |
| 6120:06 | Encoder B | | BOOLEAN | RO | 0x00 (0dec) |
| 6120:07 | Encoder C | | BOOLEAN | RO | 0x00 (0dec) |

##### Index 6160 DMC Inputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 6160:0 | DMC Inputs Ch.2 | | UINT8 | RO | 0x3C (60dec) |
| 6160:02 | DMC__FeedbackStatus__Latch extern valid | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:03 | DMC__FeedbackStatus__Set counter done | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:0D | DMC__FeedbackStatus__Status of extern latch | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:11 | DMC__DriveStatus__Ready to enable | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:12 | DMC__DriveStatus__Ready | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:13 | DMC__DriveStatus__Warning | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:14 | DMC__DriveStatus__Error | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:15 | DMC__DriveStatus__Moving positive | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:16 | DMC__DriveStatus__Moving negative | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:1C | DMC__DriveStatus__Digital input 1 | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:1D | DMC__DriveStatus__Digital input 2 | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:21 | DMC__PositioningStatus__Busy | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:22 | DMC__PositioningStatus__In-Target | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:23 | DMC__PositioningStatus__Warning | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:24 | DMC__PositioningStatus__Error | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:25 | DMC__PositioningStatus__Calibrated | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:26 | DMC__PositioningStatus__Accelerate | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:27 | DMC__PositioningStatus__Decelerate | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:28 | DMC__PositioningStatus__Ready to execute | | BOOLEAN | RO | 0x00 (0dec) |
| 6160:31 | DMC__Set position | | INT64 | RO | |
| 6160:32 | DMC__Set velocity | | INT16 | RO | 0x0000 (0dec) |
| 6160:33 | DMC__Actual drive time | | UINT32 | RO | 0x00000000 (0dec) |
| 6160:34 | DMC__Actual position lag | | INT64 | RO | |
| 6160:35 | DMC__Actual velocity | | INT16 | RO | 0x0000 (0dec) |
| 6160:36 | DMC__Actual position | | INT64 | RO | |
| 6160:37 | DMC__Error id | | UINT32 | RO | 0x00000000 (0dec) |
| 6160:38 | DMC__Input cycle counter | | UINT8 | RO | 0x00 (0dec) |
| 6160:39 | DMC__Channel id | | UINT8 | RO | 0x00 (0dec) |
| 6160:3A | DMC__Latch value | | INT64 | RO | |
| 6160:3B | DMC__Cyclic info data 1 | | INT16 | RO | 0x0000 (0dec) |
| 6160:3C | DMC__Cyclic info data 2 | | INT16 | RO | 0x0000 (0dec) |

##### Index 7001 FB Touch probe outputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7001:0 | FB Touch probe outputs Ch.1 | | UINT8 | RO | 0x0E (14dec) |
| 7001:01 | TP1 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:02 | TP1 Continous | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:03 | TP1 Trigger mode | | BIT2 | RO | 0x00 (0dec) |
| 7001:05 | TP1 Enable pos edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:06 | TP1 Enable neg edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:09 | TP2 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:0A | TP2 Continous | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:0B | TP2 Trigger mode | | BIT2 | RO | 0x00 (0dec) |
| 7001:0D | TP2 Enable pos edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7001:0E | TP2 Enable neg edge | | BOOLEAN | RO | 0x00 (0dec) |

##### Index 7010 DRV Outputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7010:0 | DRV Outputs Ch.1 | | UINT8 | RO | 0x13 (19dec) |
| 7010:01 | Controlword | Bit 0 : Switch on<br>Bit 1 : Enable voltage<br>Bit 2 : reserved<br>Bit 3 : Enable operation<br>Bit 4 - 6 : reserved<br>Bit 7 : Fault reset<br>Bit 8 - 15 : reserved | UINT16 | RO | 0x0000 (0dec) |
| 7010:03 | Modes of operation | permitted values:<br>• 8: Cyclic synchronous position mode (CSP)<br>• 9: Cyclic synchronous velocity mode (CSV)<br>• 10: Cyclic synchronous torque mode (CST)<br>• 11: Cyclic synchronous torque mode with commutation angle (CSTCA)<br>• 131: Drive Motion Control (DMC) | UINT8 | RW | 0x08 (8dec) |
| 7010:05 | Target position | | UINT32 | RO | 0x00000000 (0dec) |
| 7010:06 | Target velocity | | INT32 | RO | 0x00000000 (0dec) |
| 7010:09 | Target torque | | INT16 | RO | 0x0000 (0dec) |
| 7010:0A | Torque offset | | INT16 | RO | 0x0000 (0dec) |
| 7010:0B | Torque limitation | | UINT16 | RW | 0x7FFF (32767dec) |
| 7010:0E | Commutation angle | | UINT16 | RO | 0x0000 (0dec) |
| 7010:0F | Velocity offset | | INT32 | RO | 0x00000000 (0dec) |
| 7010:10 | Positive torque limit value | | UINT16 | RW | 0x7FFF (32767dec) |
| 7010:11 | Negative torque limit value | | UINT16 | RW | 0x7FFF (32767dec) |
| 7010:12 | Low velocity limit value | | INT32 | RW | 0x00000000 (0dec) |
| 7010:13 | High velocity limit value | | INT32 | RW | 0x00000000 (0dec) |

##### Index 7060 DMC Outputs Ch.1

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7060:0 | DMC Outputs Ch.1 | | UINT8 | RO | 0x36 (54dec) |
| 7060:02 | DMC__FeedbackControl__Enable latch extern on positive edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:03 | DMC__FeedbackControl__Set counter | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:04 | DMC__FeedbackControl__Enable latch extern on negative edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:11 | DMC__DriveControl__Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:12 | DMC__DriveControl__Reset | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:21 | DMC__PositioningControl__Execute | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:22 | DMC__PositioningControl__Emergency stop | | BOOLEAN | RO | 0x00 (0dec) |
| 7060:31 | DMC__Set counter value | | INT64 | RO | |
| 7060:32 | DMC__Target position | | INT64 | RO | |
| 7060:33 | DMC__Target velocity | | INT16 | RO | 0x0000 (0dec) |
| 7060:34 | DMC__Start type | | UINT16 | RO | 0x0000 (0dec) |
| 7060:35 | DMC__Target acceleration | | UINT16 | RO | 0x0000 (0dec) |
| 7060:36 | DMC__Target deceleration | | UINT16 | RO | 0x0000 (0dec) |

##### Index 7101 FB Touch probe outputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7101:0 | FB Touch probe outputs Ch.2 | | UINT8 | RO | 0x0E (14dec) |
| 7101:01 | TP1 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:02 | TP1 Continous | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:03 | TP1 Trigger mode | | BIT2 | RO | 0x00 (0dec) |
| 7101:05 | TP1 Enable pos edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:06 | TP1 Enable neg edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:09 | TP2 Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:0A | TP2 Continous | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:0B | TP2 Trigger mode | | BIT2 | RO | 0x00 (0dec) |
| 7101:0D | TP2 Enable pos edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7101:0E | TP2 Enable neg edge | | BOOLEAN | RO | 0x00 (0dec) |

##### Index 7110 DRV Outputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7110:0 | DRV Outputs Ch.2 | | UINT8 | RO | 0x13 (19dec) |
| 7110:01 | Controlword | Bit 0 : Switch on<br>Bit 1 : Enable voltage<br>Bit 2 : reserved<br>Bit 3 : Enable operation<br>Bit 4 - 6 : reserved<br>Bit 7 : Fault reset<br>Bit 8 - 15 : reserved | UINT16 | RO | 0x0000 (0dec) |
| 7110:03 | Modes of operation | permitted values:<br>• 8: Cyclic synchronous position mode (CSP)<br>• 9: Cyclic synchronous velocity mode (CSV)<br>• 10: Cyclic synchronous torque mode (CST)<br>• 11: Cyclic synchronous torque mode with commutation angle (CSTCA)<br>• 131: Drive Motion Control (DMC) | UINT8 | RW | 0x08 (8dec) |
| 7110:05 | Target position | | UINT32 | RO | 0x00000000 (0dec) |
| 7110:06 | Target velocity | | INT32 | RO | 0x00000000 (0dec) |
| 7110:09 | Target torque | | INT16 | RO | 0x0000 (0dec) |
| 7110:0A | Torque offset | | INT16 | RO | 0x0000 (0dec) |
| 7110:0B | Torque limitation | | UINT16 | RW | 0x7FFF (32767dec) |
| 7110:0E | Commutation angle | | UINT16 | RO | 0x0000 (0dec) |
| 7110:0F | Velocity offset | | INT32 | RO | 0x00000000 (0dec) |
| 7110:10 | Positive torque limit value | | UINT16 | RW | 0x7FFF (32767dec) |
| 7110:11 | Negative torque limit value | | UINT16 | RW | 0x7FFF (32767dec) |
| 7110:12 | Low velocity limit value | | INT32 | RW | 0x00000000 (0dec) |
| 7110:13 | High velocity limit value | | INT32 | RW | 0x00000000 (0dec) |

##### Index 7160 DMC Outputs Ch.2

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 7160:0 | DMC Outputs Ch.2 | | UINT8 | RO | 0x36 (54dec) |
| 7160:02 | DMC__FeedbackControl__Enable latch extern on positive edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:03 | DMC__FeedbackControl__Set counter | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:04 | DMC__FeedbackControl__Enable latch extern on negative edge | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:11 | DMC__DriveControl__Enable | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:12 | DMC__DriveControl__Reset | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:21 | DMC__PositioningControl__Execute | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:22 | DMC__PositioningControl__Emergency stop | | BOOLEAN | RO | 0x00 (0dec) |
| 7160:31 | DMC__Set counter value | | INT64 | RO | |
| 7160:32 | DMC__Target position | | INT64 | RO | |
| 7160:33 | DMC__Target velocity | | INT16 | RO | 0x0000 (0dec) |
| 7160:34 | DMC__Start type | | UINT16 | RO | 0x0000 (0dec) |
| 7160:35 | DMC__Target acceleration | | UINT16 | RO | 0x0000 (0dec) |
| 7160:36 | DMC__Target deceleration | | UINT16 | RO | 0x0000 (0dec) |

##### Index F000 Modular Device Profile

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F000:0 | Modular Device Profile | General information for the Modular Device Profile | UINT8 | RO | 0x02 (2dec) |
| F000:01 | Index distance | Index distance of the objects of the individual channels | UINT16 | RO | 0x0010 (16dec) |
| F000:02 | Maximum number of modules | Number of channels | UINT16 | RO | 0x0017 (23dec) |

##### Index F008 Code word

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F008:0 | Code word | | UINT32 | RW | 0x00000000 (0dec) |

##### Index F010 Module Profile List

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F010:0 | Module Profile List | | UINT8 | RO | 0x17 (23dec) |
| F010:01 | SubIndex 001 | | UINT32 | RO | 0x00000201 (513dec) |
| F010:02 | SubIndex 002 | | UINT32 | RO | 0x000002E6 (742dec) |
| F010:03 | SubIndex 003 | | UINT32 | RO | 0x00000064 (100dec) |
| F010:04 | SubIndex 004 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:05 | SubIndex 005 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:06 | SubIndex 006 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:07 | SubIndex 007 | | UINT32 | RO | 0x000002EE (750dec) |
| F010:08 | SubIndex 008 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:09 | SubIndex 009 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0A | SubIndex 010 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0B | SubIndex 011 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0C | SubIndex 012 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0D | SubIndex 013 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0E | SubIndex 014 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:0F | SubIndex 015 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:10 | SubIndex 016 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:11 | SubIndex 017 | | UINT32 | RO | 0x00000201 (513dec) |
| F010:12 | SubIndex 018 | | UINT32 | RO | 0x000002E6 (742dec) |
| F010:13 | SubIndex 019 | | UINT32 | RO | 0x00000064 (100dec) |
| F010:14 | SubIndex 020 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:15 | SubIndex 021 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:16 | SubIndex 022 | | UINT32 | RO | 0x00000000 (0dec) |
| F010:17 | SubIndex 023 | | UINT32 | RO | 0x000002EE (750dec) |

##### Index F081 Download revision

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F081:0 | Download revision | | UINT8 | RO | 0x01 (1dec) |
| F081:01 | Revision number | | UINT32 | RW | 0x00000000 (0dec) |

##### Index FB00 Command

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| FB00:0 | Command | | UINT8 | RO | 0x03 (3dec) |
| FB00:01 | Request | | OCTET-STRING[2] | RW | {0} |
| FB00:02 | Status | | UINT8 | RO | 0x00 (0dec) |
| FB00:03 | Response | | OCTET-STRING[6] | RO | {0} |

##### Index FB13 DRV Key code

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| FB13:0 | DRV Key code | | UINT8 | RO | 0x01 (1dec) |
| FB13:01 | Code | | OCTET-STRING[32] | RW | {0} |

##### Index FB40 Memory interface

| Index (hex) | Name | Meaning | Data type | Flags | Default |
| :--- | :--- | :--- | :--- | :--- | :--- |
| FB40:0 | Memory interface | | UINT8 | RO | 0x03 (3dec) |
| FB40:01 | Address | | UINT32 | RW | 0x00000000 (0dec) |
| FB40:02 | Length | | UINT16 | RW | 0x0000 (0dec) |
| FB40:03 | Data | | OCTET-STRING[8] | RW | {0} |
