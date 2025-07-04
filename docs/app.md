We're building an app that allows for the management of quickemu vms, with integration of quickget to create vms.

The main goal is an UI which allows the creation, management and visualization of quickemu vms. We're keeping this simple though, all vm information if kept in the `.conf` files that quickemu supports. This app should be able to manage a folder of `.conf` files of exising vms and it should use a default `.config` directory.

We want a modern minimalistic UI that fits in both Linux (GNOME) and Macos. We want nice graphics which displays CPU/RAM usage per VM. Starting a VM should launch the native display of that VM. We shuold be able to manage already running VMs.

It should support quickemu/quickget download if it can't be found in the PATH. Ideally the whole app is one binary. A WebUI would be very interesting. 

The app should auto-scan the folder that is selected. THe folder selection can happen in the UI or in a config file, whatever is more native to the framework being used.