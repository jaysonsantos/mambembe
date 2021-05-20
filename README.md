# mambembe

My thanks to [alexzorin/authy](https://github.com/alexzorin/authy) as a bunch of the process is documented there.

## Description

This is an implementation of an authy client, so far only the command line is implemented but the idea is to have also a desktop client using [iced](https://github.com/hecrj/iced).
The main logic of authy should happen inside the lib as it can be reused by other components.

## cli usage

```
> mambembe-cli --help

mambembe-cli 0.1.1

USAGE:
    mambembe-cli <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    get-token
    help               Prints this message or the help of the given subcommand(s)
    list-services
    register-device
```

The basic flow is calling `register-device` so mambembe can record your access token and save it to your keyring service (Keychain on macOS, secrets-manager on linux which is baked by KWallet or gnome-keyring, or Windows Vault).
To register your device call `mambembe-cli register-device --device-name <device-name> --phone <phone>` where **IMPORTANT** phone has to be in a specific format (as there is no cleaning in place) e.g.: `49-123456`, where `49` is the country code and `123456` is your phone.
To refresh your tokens you can call `list-services` and it will always hit authy's API to get the list of your current devices.
To get a token you can call `mambembe-cli get-token --service-name <service-name>` where `<service-name>` can be a partial as it will make a fuzzy search, e.g.:

```
mambembe-cli get-token --service-name gh
Service: "github.com/jaysonsantos" Token: "123456" Type: 1
```

### useful aliases

an alias to simply call `mg gh` to get all tokens that partially match with `gh`

```
alias mg='mambembe-cli get-token -s'
```

a function to get a list of tokens and feed to `fzf` and then copy to the clipboard

```
mgc ()
{
    mambembe-cli get-token -s "$@" | fzf --reverse -0 -1 | rg -oP 'Token: "\K\d+' | xclip -i -selection clipboard
}
```
