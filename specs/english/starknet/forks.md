# Fork Protocol

This is **WORK IN PROGRESS**.

Summary:

- The Fork Protocol is triggered by the expiration of deadlines in the
  [Validator Set Updates Protocol][valset-spec].
- The goal of this protocol is to **enforce** the adoption of a new validator
  set, produced by L1, by Starknet (L2)
  - This is why the Starkware team uses _reset_ to refer to this procedure,
    that resets the validator set
- The "Rainy Day - Reset" scenario of the [Starknet specification][starkware-spec]
  illustrates the operation of the Fork Protocol
- The validator set _reset_ is represented by orange (?) arrows in diagrams 
  in the [Starknet specification][starkware-spec]
  - Notice, however, that there is no explicit signalization from L1 to start
    the reset protocol
  - Instead, validators that remain in the reset validator set and nodes that
    become validators in the reset validator set are expected to initiate the
    Fork Protocol, once they realize that it is needed
- The reset of the validator set produces a fork of Starknet blockchain:
  - The initial state of the fork is derived from the latest block of the
    previous fork that has been successfully proven.
    By successfully here we mean that the produced proof was accepted by L1.
  - The validator set for the fork is the last validator set accepted by L1
    plus all the "stale registrations", i.e., validator set updates produced by
    L1 but not included in Starknet blocks (L2) in due time.
    - This is not enough precise yet, for instance, how the validator set
      updates committed to blocks prior to the fork are handled.
      Namely, how the value of the state variable `S`, defined in
      the [Validator Set Updates Protocol][valset-spec] specification,
      in the first block `H_0` of the fork can be consistent with the latest
      state `S(H_0 - 1)` from the previous fork. 
      A possibility is to add validator updates to block `H_0` in the new fork
      (playing the role of its "genesis") in order to preserve some consistency.
  - For all effects, it can be useful to consider the first block of a new fork
    as it was a genesis state or block.
  - The execution and state of the blockchain after the height, or the block
    from where the fork is derived is irrelevant. Refer to the Enforcement
    section of the [Validator Set Updates Protocol][valset-spec] for details.
  - It is assumed that nodes joining the validator set of a fork have access to
    all state they need to produce and validate blocks (i.e., make progress) in
    that fork. Should we blindly stick to this assumption?
- Different forks are uniquely identified by a _ForkId_:
  - The _ForkId_ is set to the last "Ethereum Validator Epoch" _EVE_ which "has
    the most recent stale update".
    - Apparently, this should indeed produce unique *ForkId*s.
  - Consensus messages, and the Starknet communication in general, should
    include the _ForkId_, in order to distinguish content, messages, and data
    belonging to different executions, or forks.

[valset-spec]: ./validator_set.md
[starkware-spec]: https://docs.google.com/document/d/1OaYLh9o10DIsGpW0GTRhWl-IJiVyjRsy7UttHs9_1Fw
