# gitfetch

A [neofetch](https://github.com/dylanaraps/neofetch) inspired tool for fetching
github profiles in the terminal

## Note

It is recommended to pass in a github token via the `--token MY_TOKEN` argument.
You can use gitfetch without it, however you may be rate limited, due to the
fact of github only accepting 60 requests per **Hour**.

## Building

```Rust
fn main(){
    todo!()
}
```

## Usage

To use gitfetch without a token (keep in mind, a username is required to be
passed via the `-u` or `--user` flag) `gitfetch -- -u EinSatzMitX`

If you want to make sure not to get rate limited, pass in a token like this

`gitfetch -- -t MY_TOKEN -u EinSatzMitX`

`gitfetch -- --token "$(cat path_to_token.txt)" -u EinSatzMitX`

## Examples

<img src ="example1.png"/>
