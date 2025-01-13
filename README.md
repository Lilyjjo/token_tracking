# Token Activity Tracker

This is a tool to visualize token activity on a given pool and token.

## Usage

1. Copy the `local.env.example` file to `.env` and set the environment variables (can use `just copy-env` to do this)
2. Run the program with the desired mode.

### For live processing
```bash
just live
```

### For single block processing
```bash
just single 24985835
```

### To toggle log level
```bash
just live debug
```