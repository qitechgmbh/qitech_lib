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


