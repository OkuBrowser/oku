This library offers a lightweight discovery service based on mDNS.
It is geared towards facilitating new peers discovering a swarm in the local network to allow joining to be implemented without needing a preconfigured contact point.
The difference to using plain mDNS is that this library carefully chooses when to send queries or responses such that regardless of swarm size the used network bandwidth does not grow out of bounds.
At the same time, it maintains a steady flow of packets which can serve as an indicator of liveness and connectedness.

## Algorithm

Configuration parameters: τ (discovery time target), φ (response frequency target)

Each node tracks the members of the local swarm by using every seen mDNS response as a liveness signal for its sender.
This yields an estimate S for the swarm size.
Since φ is the expected response rate, the long-term average response frequency for a given peer is φ ÷ S.
To account for jitter due to the random nature of response generation described below, we prune a peer once it has not been seen for longer than 3S ÷ φ.

Each node sends mDNS queries according to the following algorithm:

1. set timeout to a duration pulled at random from the interval [τ, τ + (S + 1) × τ ÷ 10)
2. upon reception of another node’s mDNS query go to response mode
3. upon timeout, send mDNS query and go to response mode

Each node responds to a mDNS query according to the following algorithm:

1. set timeout to a duration `random` + `extra`
    - `random` is pulled from the interval [0, 100ms * (S + 1) ÷ (τ × φ))
    - `extra` is set to 100ms * min(10, S ÷ (τ × φ)) if the peer has sent a response in the previous cycle
    - `extra` is set to max(0, `extra` - 100ms) if the peer has not sent a response in the previous cycle
2. set counter to zero
3. upon reception of another node’s mDNS response to this query, increment the counter
4. when counter is greater than τ•φ go to query mode (note that this assumes that τ•φ>1 for proper operation)
5. upon timeout send response and go to query mode

Responses received in either mode are used to discover peers.

## Discussion

The probability density function of the mean of N samples from a uniform distribution of [0, 1) is (1-x)^(N-1).
(To see this, consider that for a given minium x the probability of the other values being greater than x is given by the aforementioned expression.)
The zeroth moment of this is 1 ÷ n and the first moment is 1 ÷ (n(n+1)), wherefore the mean is 1 ÷ (n + 1).

The mean of the earliest timeout in query mode assuming a correct estimate for the swarm size S thus is 1.1τ.
Spreading out the drawn timeouts linearly with the swarm size ensures that it is quite unlikely that two peers concurrently send a query (assuming that the cadence is much larger than typical network latencies).

Similarly, the mean of the earliest response timeout (neglecting `extra` delay) is 100ms ÷ (τ•φ), which roughly means that the expected number of responses should be sent in around 100ms on average.
The full query–response cycle time is therefore approximately 1.1τ + 100ms, in which time τ•φ responses plus a small number of duplicates will be received (probability of a duplicate is roughly latency ÷ 100ms).

Assuming 10% duplicates we arrive at a response frequency of 1.1τ•φ ÷ (1.1τ + 100ms) which is smaller than our response frequency target of φ.

### Newly joined nodes

In an established swarm all peers will have reasonably accurate estimates of S already, making the above calculations work.
When a fresh node joins, it assumes swarm size 1, which implies that its query timeout will be from [τ, 1.2τ) and its response timeout from [0, 200ms ÷ (τ•φ) ).
It will thus like initiate the query and also likely send the first response, making itself known to the swarm.
The `extra` delay in the next round is then calculated with a swarm size that is likely at least as large as τ•φ, meaning that the new node will be delayed by one extra response interval.
This gives priority to other nodes announcing themselves to the new member.

The `extra` delay also solves potential fairness issues when multiple fresh nodes join at the same time, giving each one of them a good chance to introduce themself to the swarm within the first few rounds.

## mDNS usage

The facilities described in [RFC6762](https://datatracker.ietf.org/doc/html/rfc6762) and [RFC6763](https://datatracker.ietf.org/doc/html/rfc6763) are used as follows:

- configurable service name NAME
- queries sent for PTR records of the form `_NAME._udp.local.` (TCP analog)
- responses give SRV records of the form `PEER_ID._NAME._udp.local.` -> `PEER_ID.local.` (and associated A/AAAA records)
- responses may include a TXT record with the same name as the SRV record, containing a list of key-value attribute pairs as described in [RFC 6763](https://datatracker.ietf.org/doc/html/rfc6763#section-6)

If multiple ports are associated to their respective IP addresses, there will be one SRV record per port pointing to a resource name of the form `PEER_ID-PORT._NAME._udp.local`.
