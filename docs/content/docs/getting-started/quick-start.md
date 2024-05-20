+++
title = "Quick Start"
description = "One page summary of how to start a new AdiDoks project."
date = 2021-05-01T08:20:00+00:00
updated = 2021-05-01T08:20:00+00:00
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "One page summary of how to start a new biliup project."
toc = true
top = false
+++

## Requirements

Before using the biliup, you need to install the [python](https://www.python.org/) ≥ 3.8.

## Run the biliup Directly

```bash
pipx install biliup
biliup
```

Visit `http://127.0.0.1:19159/` in the browser.

## Installation

Just earlier we showed you how to run the biliup directly. Now we start to
install the biliup with config step by step.

### Step 1: Create a new directory

```bash
mkdir biliup
cd biliup
```

### Step 2: Install biliup

Download this theme to your PATH directory:

```bash
pip install biliup
```

Or install by docker:

```bash
docker pull ghcr.io/biliup/caution:latest
```

### Step 3: Configuration

Create config in your `config.toml` in the biliup derectory:

```toml
# 以下为必填项
[streamers."1xx直播录像"] # 设置直播间1
url = ["https://www.twitch.tv/1xx"]
tags = ["biliup"]

# 设置直播间2
[streamers."2xx直播录像"]
url = ["https://www.twitch.tv/2xx"]
tags = ["biliup"]   
```

Or use the `config.yaml` from the github directory to your project's
root directory:

```bash
vim config.yaml
```

```yaml
streamers:
    xxx直播录像:
        url:
            - https://www.twitch.tv/xxx
        tags: biliup
```


### Step 4: Run the project

Just run `biliup start` in the root path of the project:

```bash
biliup start
```
