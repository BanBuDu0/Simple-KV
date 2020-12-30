# Simple-KV

## Use raft-rs, grpc-rs to build a simple HA key-value service

+ Provide basic get, set, delete, and scan operations
+ The data saved in one node must be persistent in the disk (restart can't lose data)
+ Need to show - Kill minority node, the service can still work
+ Need to show - Kill majority node, the service can't work
+ Need to support add/remove node dynamically
+ [Plus Point] Use a benchmark tool to find some performance problems.

## System Architecture

![SystemArchitecture.png](https://i.loli.net/2020/12/25/jCc1ukvneVWDfdI.png)
+ WAL Method

  Apply log的时候先写日志，再定时刷盘

+ Follower Read

  Client get从Follower读取数据

## TIMELINE

| TIME  | TODO                                       | CHECK |
| ----- | :----------------------------------------- | ----- |
| 12.25 | 整体项目思考                 | √     |
| 12.26 | 学习rust和grpc-rs和raft-rs，编写grpc proto |  √       |
| 12.27 | 编写client和server                         |       |
| 12.28 | 编写client和server，编写WAL和Follower read |       |
| 12.29 | Leader & Do benchmark                |       |
| 12.30 | Deadline                                   |       |

## RUN

+ cargo run --manifest-path Cargo.toml --bin server
+ cargo run --manifest-path Cargo.toml --bin client 34431

## mind

Use ```agree_chs: HashMap<i64, Receiver<Proposal>>``` to save the client -> Proposal Receiver. When a Proposal
is Applied, the Receiver will receive the Proposal, and notify the blocked grpc request.


