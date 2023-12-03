## Change directory, quickly

Are you tired of changing directories in a big files trees? If so, **cdq** comes with a rescue.

Let's say you have a following structure 
```bash
➜  foo tree
.
└── foo
    └── bar
        └── foobar
            └── edit_me.xyz
```

If you want to quickly go to foobar directory to edit something there, you do not need to type the entire path with 
`cd foo/bar/foobar`. With **cdq** you simply can run `cdq foobar`!

> Note: It is an incredibly basic MVP, but I plan to make it better every time I have some time to spare.

### Installation

As of now it is not integrated with shell too well, so the process is unfortunately a bit cumbersome.

- Clone the repository: `git clone https://github.com/wzslr321/cdq`
- `cd cdq && cargo build -r`
- Add cdq function to your shell config file, to execute the program and interpret its output

Example for `.zshrc`
```sh
cdq() {
    local output=$(~/Remi/rust/cdq/target/release/cdq $1)
    echo $output
    local dir=$(echo $output | awk -F'Path=' '{print $2}')
    if [ -d "$dir" ]; then
        echo "Proceeding to the $dir"
        cd "$dir"
    fi
}

```

- Refresh shell - `exec zsh`
- Run `cdq --version` to make sure it works!