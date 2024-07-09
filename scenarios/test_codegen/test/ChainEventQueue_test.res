open RescriptMocha

let eventMock1: Types.event = Gravatar_NewGravatar({
  blockNumber: 1,
  chainId: 54321,
  blockHash: "0xdef",
  logIndex: 0,
  params: MockEvents.newGravatar1,
  blockTimestamp: 1900000,
  srcAddress: "0x1234512345123451234512345123451234512345"->Ethers.getAddressFromStringUnsafe,
  transactionHash: "0xabc",
  transactionIndex: 987,
  txOrigin: None,
  txTo: None,
})

let qItemMock1: Types.eventBatchQueueItem = {
  timestamp: 0,
  chain: MockConfig.chain1337,
  blockNumber: 1,
  logIndex: 0,
  event: eventMock1,
  eventMod: module(Types.Gravatar.NewGravatar),
}

let eventMock2: Types.event = Gravatar_NewGravatar({
  blockNumber: 2,
  chainId: 54321,
  blockHash: "0xabc",
  logIndex: 1,
  params: MockEvents.newGravatar2,
  blockTimestamp: 1900001,
  srcAddress: "0x1234512345123451234512345123451234512346"->Ethers.getAddressFromStringUnsafe,
  transactionHash: "0xdef",
  transactionIndex: 988,
  txOrigin: None,
  txTo: None,
})

let qItemMock2: Types.eventBatchQueueItem = {
  timestamp: 1,
  chain: MockConfig.chain1337,
  blockNumber: 2,
  logIndex: 1,
  event: eventMock1,
  eventMod: module(Types.Gravatar.NewGravatar),
}

describe("Chain Event Queue", () => {
  Async.it("Awaits item to be pushed to queue before resolveing", async () => {
    let q = ChainEventQueue.make(~maxQueueSize=100)

    let itemPromise = q->ChainEventQueue.popSingleAndAwaitItem

    //pop backlog callbacks should have 1 item in the queue since we are
    //waiting for an item to pop
    Assert.equal(q.popBacklogCallbacks->SDSL.Queue.size, 1)

    await q->ChainEventQueue.awaitQueueSpaceAndPushItem(qItemMock1)

    //Pop backlog callbacks should have 0 in the queue since pushing an item
    //should have remove/run that awaiting callback
    Assert.equal(q.popBacklogCallbacks->SDSL.Queue.size, 0)
    let poppedItem = await itemPromise

    Assert.deepEqual(~message="Poped item not the same", qItemMock1, poppedItem)
    Assert.equal(q.queue->SDSL.Queue.size, 0)
  })

  Async.it("Awaits space on the queue before pushing", async () => {
    let hasResolvedPromise = ref(false)
    //Make a queue with small max size
    let q = ChainEventQueue.make(~maxQueueSize=1)
    //Fill the queue to max size
    await q->ChainEventQueue.awaitQueueSpaceAndPushItem(qItemMock1)
    //Try push an item to the queu
    let nextIemPromise =
      q
      ->ChainEventQueue.awaitQueueSpaceAndPushItem(qItemMock2)
      ->Js.Promise2.then(
        a => {
          hasResolvedPromise := true
          a->Js.Promise2.resolve
        },
      )
    //Assert that the item is not on the queue
    Assert.deepEqual(q->ChainEventQueue.peekFront, Some(qItemMock1))
    Assert.equal(
      q.queue->SDSL.Queue.size,
      1,
      ~message="queue should start with a size of 1 even though 2 items have been pushed to it",
    )

    //Pop an item off
    Assert.equal(hasResolvedPromise.contents, false)
    Assert.equal(q.pushBacklogCallbacks->SDSL.Queue.size, 1)

    let popedValue1 = q->ChainEventQueue.popSingle
    Assert.deepEqual(popedValue1, Some(qItemMock1))
    Assert.equal(q.pushBacklogCallbacks->SDSL.Queue.size, 0)

    await nextIemPromise

    Assert.equal(
      q.queue->SDSL.Queue.size,
      1,
      ~message="The queue should stay at size 1 because it should immediately get the new value",
    )

    //assert that the front of the queue is the new item
    let popedValue2 = q->ChainEventQueue.popSingle
    Assert.deepEqual(popedValue2, Some(qItemMock2))
  })
})
