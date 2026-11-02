cargo build --example $1
cp /tmp/qit-cargo-target/debug/examples/$1 ./target/
sudo setcap 'cap_net_raw,cap_sys_nice,cap_ipc_lock=eip' ./target/$1
mkdir target
./target/$1 $2
