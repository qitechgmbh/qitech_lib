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
