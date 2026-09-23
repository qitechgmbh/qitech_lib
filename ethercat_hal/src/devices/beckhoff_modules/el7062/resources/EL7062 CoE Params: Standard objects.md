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
