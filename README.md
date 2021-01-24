# doorstate-rs

A rewrite with a few more features of [this](https://github.com/hackspace-marburg/spaceapi).

Modify our hackspace's [door](https://hsmr.cc/Infrastruktur/Door) state with a Raspberry Pi and a ridiculously oversized switch. This script writes the `spaceapi.json`  and `Site.SiteNav` files for the [Space API](spaceapi.net) and [PmWiki](https://www.pmwiki.org). These files are mounted via sshfs.

## Installation instructions

Make sure the root user is able to log in the remote machine by having the
`~/.ssh` folder prepared. Furthermore, modify the `/etc/fstab` file to use
*sshfs* as documented
[here](https://wiki.archlinux.org/index.php/Sshfs#Automounting).

Optional, requires feature `gpio-support`: Connect the switch to a gpio pin and a ground pin. Set this pin as parameter `-s PINNUMBER` in the environment file (doorstate-rs).

```bash
# Install required software
# (Probably only pigpio)
# and have rust installed

# Clone this repository and cd into it
git clone https://github.com/hackspace-marburg/doorstate-rs.git
cd doorstate-rs
sudo cp doorstate-rs.service /etc/systemd/system/

# Drop --features gpio-support if not in use on an Rasberry Pi.
cargo build --features gpio-support --release


sudo mkdir /etc/conf.d/
sudo cp doorstate-rs /etc/conf.d/
# Modify /etc/conf.d/doorstate-rs afterwards

sudo systemctl daemon-reload
sudo systemctl enable doorstate-rs.service
sudo systemctl start doorstate-rs.service
```

## Build instructions

To get a server side service which recieves MQTT messages and changes the spaceapi as well as the door state on the hsmr homepage you can simply

```bash
cargo build --release
```

If you require support for Raspberry Pi GPIO to attach a hardware door switch to it you'll need to:

```bash
cargo build --features gpio-support --release
```

Sadly all this is in dependency hell, as the used mqtt library itself had a ton of dependencies.

