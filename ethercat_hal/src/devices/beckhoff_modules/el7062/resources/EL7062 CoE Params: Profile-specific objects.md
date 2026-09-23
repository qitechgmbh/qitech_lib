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
