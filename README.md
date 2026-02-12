# QiTech-Lib
Contains basic building blocks for creating machines with EtherCAT, Serial,Modbus-RTU,Modbus-TCP etc

# Ethercat wrapper
Instead of needing to include ethercrab subdevices/logic EVERYWHERE
Just have a thread running with a state machine for Ethercat, where state changes are triggered by the user through a channel.

For Ethercrab the full process data map is reconstructed by writing subdevices io_raw into a buffer, which is double buffered.
Meaning we can access the data without any locking at all and also write the data.
In theory our EthercatDevice simply take a slice of the total process data, the logic doesnt really change at all.

State changes happen by the user requesting them, either over an atomic variable or channel.
Atomic is probably better just to avoid needing async everywhere ... 
