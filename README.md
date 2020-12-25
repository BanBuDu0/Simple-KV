# Simple-KV
## Use raft-rs, grpc-rs to build a simple HA key-value service

+ Provide basic get, set, delete, and scan operations

+ The data saved in one node must be persistent in the disk (restart can't lose data)

+ Need to show - Kill minority node, hte service can  still work

+ Need to show - Kill majority node, the service can't work

+ Need to support add/remove node dynamically

+ [Plus Point] Use a benchmark tool to find some performance problems.

## DEADLINE

12.30