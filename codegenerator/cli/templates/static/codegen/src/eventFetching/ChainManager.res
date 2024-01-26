type t = {
  chainFetchers: ChainMap.t<ChainFetcher.t>,
  //The priority queue should only house the latest event from each chain
  //And potentially extra events that are pushed on by newly registered dynamic
  //contracts which missed being fetched by they chainFetcher
  arbitraryEventPriorityQueue: SDSL.PriorityQueue.t<Types.eventBatchQueueItem>,
}

let getComparitorFromItem = (queueItem: Types.eventBatchQueueItem) => {
  let {timestamp, chain, blockNumber, logIndex} = queueItem
  EventUtils.getEventComparator({
    timestamp,
    chainId: chain->ChainMap.Chain.toChainId,
    blockNumber,
    logIndex,
  })
}

let getEventQueuePeakComparitor = (v: ChainFetcher.eventQueuePeek) => {
  switch v {
  | Item(i) => i->getComparitorFromItem
  | NoItem(latestFetchedBlockTimestamp, chainId) => (
      latestFetchedBlockTimestamp,
      chainId->ChainMap.Chain.toChainId,
      0,
      0,
    )
  }
}

let priorityQueueComparitor = (a: Types.eventBatchQueueItem, b: Types.eventBatchQueueItem) => {
  if a->getComparitorFromItem < b->getComparitorFromItem {
    -1
  } else {
    1
  }
}

let chainFetcherPeekComparitorEarliestEvent = (
  a: ChainFetcher.eventQueuePeek,
  b: ChainFetcher.eventQueuePeek,
): bool => {
  a->getEventQueuePeakComparitor < b->getEventQueuePeakComparitor
}

// This is similar to `chainFetcherPeekComparitorEarliestEvent`, but it prioritizes events over `NoItem` no matter what the timestamp of `NoItem` is.
let chainFetcherPeekComparitorEarliestEventPrioritizeEvents = (
  a: ChainFetcher.eventQueuePeek,
  b: ChainFetcher.eventQueuePeek,
): bool => {
  switch (a, b) {
  | (Item(itemA), Item(itemB)) => itemA->getComparitorFromItem < itemB->getComparitorFromItem
  | (Item(_), NoItem(_, _)) => true
  | (NoItem(_, _), Item(_)) => false
  | (
      NoItem(latestFetchedBlockTimestampA, chainIdA),
      NoItem(latestFetchedBlockTimestampB, chainIdB),
    ) =>
    (latestFetchedBlockTimestampA, chainIdA) < (latestFetchedBlockTimestampB, chainIdB)
  }
}

type nextEventErr = NoItemsInArray

let createDetermineNextEventFunction = (~isUnorderedHeadMode: bool): (
  array<ChainFetcher.eventQueuePeek> => result<ChainFetcher.eventQueuePeek, nextEventErr>
) => {
  let comparitorFunction = if isUnorderedHeadMode {
    chainFetcherPeekComparitorEarliestEventPrioritizeEvents
  } else {
    chainFetcherPeekComparitorEarliestEvent
  }

  (chainFetchersPeeks: array<ChainFetcher.eventQueuePeek>) => {
    let nextItem = chainFetchersPeeks->Belt.Array.reduce(None, (accum, valB) => {
      switch accum {
      | None => Some(valB)
      | Some(valA) =>
        if comparitorFunction(valA, valB) {
          Some(valA)
        } else {
          Some(valB)
        }
      }
    })

    switch nextItem {
    | None => Error(NoItemsInArray)
    | Some(item) => Ok(item)
    }
  }
}

let determineNextEvent = createDetermineNextEventFunction(
  ~isUnorderedHeadMode=Config.isUnorderedHeadMode,
)

let make = (~configs: Config.chainConfigs, ~maxQueueSize, ~shouldSyncFromRawEvents: bool): t => {
  let chainFetchers = configs->ChainMap.map(chainConfig => {
    let lastBlockScannedHashes = ReorgDetection.LastBlockScannedHashes.empty(
      ~confirmedBlockThreshold=chainConfig.confirmedBlockThreshold,
    )

    ChainFetcher.make(
      ~chainConfig,
      ~maxQueueSize,
      ~shouldSyncFromRawEvents,
      ~lastBlockScannedHashes,
    )
  })

  {
    chainFetchers,
    arbitraryEventPriorityQueue: SDSL.PriorityQueue.makeAdvanced([], priorityQueueComparitor),
  }
}

exception NotASourceWorker
let toSourceWorker = (worker: ChainWorker.chainWorker): SourceWorker.sourceWorker =>
  switch worker {
  | HyperSync(w) => HyperSync(w)
  | Rpc(w) => Rpc(w)
  | RawEvents(_) => NotASourceWorker->raise
  }

//TODO this needs to action a roll back
let reorgStub = async (chainManager: t, ~chainFetcher: ChainFetcher.t, ~lastBlockScannedData) => {
  //get a list of block hashes via the chainworker
  let blockNumbers =
    chainFetcher.lastBlockScannedHashes->ReorgDetection.LastBlockScannedHashes.getAllBlockNumbers
  let blockNumbersAndHashes =
    await chainFetcher.chainWorker.contents
    ->toSourceWorker
    ->SourceWorker.getBlockHashes(~blockNumbers)
    ->Promise.thenResolve(Belt.Result.getExn)

  let rolledBack =
    chainFetcher.lastBlockScannedHashes
    ->ReorgDetection.LastBlockScannedHashes.rollBackToValidHash(~blockNumbersAndHashes)
    ->Belt.Result.getExn

  let reorgStartPlace = rolledBack->ReorgDetection.LastBlockScannedHashes.getLatestLastBlockData

  switch reorgStartPlace {
  | Some({blockNumber, blockTimestamp}) => () //Rollback to here
  | None => () //roll back to start of chain. Can't really happen
  }
  //Stop workers
  //Roll back
  //Start workers again
  let _ = chainManager
  Logging.warn({
    "msg": "A Reorg Has occurred",
    "chainId": chainFetcher.chainConfig.chain->ChainMap.Chain.toChainId,
    "lastBlockScannedData": lastBlockScannedData,
  })
}

/**
For each chain with a confirmed block threshold, find the earliest block ranged scanned that exists
in that threshold

Returns None in the case of no block range entries and in the case that there is only 1 chain since
there is no need to consider other chains with prunining in this case
*/
let getEarliestMultiChainTimestampInThreshold = (chainManager: t) => {
  chainManager.chainFetchers
  ->ChainMap.values
  ->Belt.Array.map((cf): ReorgDetection.LastBlockScannedHashes.currentHeightAndLastBlockHashes => {
    lastBlockScannedHashes: cf.lastBlockScannedHashes,
    currentHeight: cf.chainWorker.contents->ChainWorker.getCurrentBlockHeight,
  })
  ->ReorgDetection.LastBlockScannedHashes.getEarliestMultiChainTimestampInThreshold
}

/**
Checks whether reorg has accured by comparing the parent hash with the last saved block hash.
*/
let hasReorgOccurred = (chainFetcher: ChainFetcher.t, ~parentHash) => {
  let recentLastBlockData =
    chainFetcher.lastBlockScannedHashes->ReorgDetection.LastBlockScannedHashes.getLatestLastBlockData

  switch (parentHash, recentLastBlockData) {
  | (None, None) => false
  | (None, Some(_)) | (Some(_), None) => true
  | (Some(parentHash), Some({blockHash})) => parentHash == blockHash
  }
}

/**
Adds latest "lastBlockScannedData" to LastBlockScannedHashes and prunes old unneeded data
*/
let addLastBlockScannedData = (
  chainFetcher: ChainFetcher.t,
  ~chainManager: t,
  ~lastBlockScannedData: ReorgDetection.lastBlockScannedData,
  ~currentHeight,
) => {
  let earliestMultiChainTimestampInThreshold =
    chainManager->getEarliestMultiChainTimestampInThreshold

  chainFetcher.lastBlockScannedHashes =
    chainFetcher.lastBlockScannedHashes
    ->ReorgDetection.LastBlockScannedHashes.pruneStaleBlockData(
      ~currentHeight,
      ~earliestMultiChainTimestampInThreshold?,
    )
    ->ReorgDetection.LastBlockScannedHashes.addLatestLastBlockData(~lastBlockScannedData)
}

/**
A function that gets partially applied as it is handed down from
chain manager to chain fetcher to chain worker
*/
let checkHasReorgOccurred = (
  chainManager: t,
  chainFetcher: ChainFetcher.t,
  lastBlockScannedData,
  ~parentHash,
  ~currentHeight,
) => {
  let hasReorgOccurred = chainFetcher->hasReorgOccurred(~parentHash)

  if hasReorgOccurred {
    chainManager->reorgStub(~chainFetcher, ~lastBlockScannedData)->ignore
  } else {
    chainFetcher->addLastBlockScannedData(~chainManager, ~currentHeight, ~lastBlockScannedData)
  }
}

let startFetchers = (self: t) => {
  self.chainFetchers
  ->ChainMap.values
  ->Belt.Array.forEach(fetcher => {
    //Start the fetchers
    fetcher
    ->ChainFetcher.startFetchingEvents(~checkHasReorgOccurred=self->checkHasReorgOccurred)
    ->ignore
  })
}

let getChainFetcher = (self: t, ~chain: ChainMap.Chain.t): ChainFetcher.t => {
  self.chainFetchers->ChainMap.get(chain)
}

//Synchronus operation that returns an optional value and will not wait
//for a value to be on the queue
//TODO: investigate can this function + Async version below be combined to share
//logic
let popBatchItem = (self: t): option<Types.eventBatchQueueItem> => {
  //Peek all next fetched event queue items on all chain fetchers
  let peekChainFetcherFrontItems =
    self.chainFetchers
    ->ChainMap.values
    ->Belt.Array.map(fetcher => fetcher->ChainFetcher.peekFrontItemOfQueue)

  //Compare the peeked items and determine the next item
  let nextItemFromBuffer = peekChainFetcherFrontItems->determineNextEvent->Belt.Result.getExn

  //Callback for handling popping of chain fetcher events
  let popNextItem = () => {
    switch nextItemFromBuffer {
    | ChainFetcher.NoItem(_, _) => None
    | ChainFetcher.Item(batchItem) =>
      //If there is an item pop it off of the chain fetcher queue and return
      let fetcher = self->getChainFetcher(~chain=batchItem.chain)
      fetcher->ChainFetcher.popQueueItem
    }
  }

  //Peek arbitraty events queue
  let peekedArbTopItem = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.top

  switch peekedArbTopItem {
  //If there is item on the arbitray events queue, pop the relevant item from
  //the chain fetcher queue
  | None => popNextItem()
  | Some(peekedArbItem) =>
    //If there is an item on the arbitrary events queue, compare it to the next
    //item from the chain fetchers
    let arbItemIsEarlier = chainFetcherPeekComparitorEarliestEvent(
      ChainFetcher.Item(peekedArbItem),
      nextItemFromBuffer,
    )

    //If the arbitrary item is earlier, return that
    if arbItemIsEarlier {
      Some(
        //safely pop the item since we have already checked there's one at the front
        self.arbitraryEventPriorityQueue
        ->SDSL.PriorityQueue.pop
        ->Belt.Option.getUnsafe,
      )
    } else {
      //Else pop the next item from chain fetchers
      popNextItem()
    }
  }
}

let getChainIdFromBufferPeekItem = (peekItem: ChainFetcher.eventQueuePeek) => {
  switch peekItem {
  | ChainFetcher.NoItem(_, chainId) => chainId
  | ChainFetcher.Item(batchItem) => batchItem.chain
  }
}
let getBlockNumberFromBufferPeekItem = (peekItem: ChainFetcher.eventQueuePeek) => {
  switch peekItem {
  | ChainFetcher.NoItem(_, _) => None
  | ChainFetcher.Item(batchItem) => Some(batchItem.blockNumber)
  }
}

type blockGroupedBatchItems = array<Types.eventBatchQueueItem>
let rec getAllBlockLogs = (
  self: t,
  bufferItem: ChainFetcher.eventQueuePeek,
  blockGroupedBatchItems: blockGroupedBatchItems,
) => {
  // Js.log({
  //   "blockGroupedBatchItems": blockGroupedBatchItems->Belt.Array.map(item => item.event),
  // })
  //Callback for handling popping of chain fetcher events
  let popNextItemOnChainQueueAndRecurse = () => {
    switch bufferItem {
    | ChainFetcher.NoItem(_, _) =>
      switch blockGroupedBatchItems {
      | [] => None
      | _ => Some(blockGroupedBatchItems)
      }

    | ChainFetcher.Item(batchItem) =>
      //If there is an item pop it off of the chain fetcher queue and return
      let fetcher = self->getChainFetcher(~chain=batchItem.chain)
      let _ = fetcher->ChainFetcher.popQueueItem

      let nextBlockGroupedBatchItems = blockGroupedBatchItems->Belt.Array.concat([batchItem])

      let peakedNextItem = fetcher->ChainFetcher.peekFrontItemOfQueue

      switch peakedNextItem {
      | ChainFetcher.NoItem(_, _) =>
        switch nextBlockGroupedBatchItems {
        | [] => None
        | _ => Some(nextBlockGroupedBatchItems)
        }
      | ChainFetcher.Item(peakedNextChainItem) =>
        if peakedNextChainItem.blockNumber == batchItem.blockNumber {
          // Js.log("continuing in recurse - sam block")
          getAllBlockLogs(self, peakedNextItem, nextBlockGroupedBatchItems)
        } else {
          // Js.log("Return early - next event is in different block.")
          Some(nextBlockGroupedBatchItems)
        }
      }
    }
  }

  //Peek arbitraty events queue
  let peekedArbTopItem = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.top

  switch peekedArbTopItem {
  //If there is item on the arbitray events queue, pop the relevant item from
  //the chain fetcher queue
  | None =>
    // Js.log("The peekedArbTopItem is EMPTY")
    //Take next item from the given nextItem from buffer
    popNextItemOnChainQueueAndRecurse()
  | Some(peekedArbItem) =>
    // Js.log({"arbPeek": peekedArbItem.event})
    let arbEventIsInSameBlock =
      (peekedArbItem.chain, Some(peekedArbItem.blockNumber)) ==
        (bufferItem->getChainIdFromBufferPeekItem, bufferItem->getBlockNumberFromBufferPeekItem)
    let arbItemIsEarlier = chainFetcherPeekComparitorEarliestEvent(
      ChainFetcher.Item(peekedArbItem),
      bufferItem, //nextItemFromBuffer,
    )

    // Js.log({
    //   "arbEventIsInSameBlock": arbEventIsInSameBlock,
    //   "arbItemIsEarlier": arbItemIsEarlier,
    // })

    if arbEventIsInSameBlock && arbItemIsEarlier {
      // Js.log("arb item is in same block but earlier")
      let _ = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.pop

      let nextBlockGroupedBatchItems = blockGroupedBatchItems->Belt.Array.concat([peekedArbItem])

      let fetcher = self->getChainFetcher(~chain=bufferItem->getChainIdFromBufferPeekItem)
      let peakedNextItem = fetcher->ChainFetcher.peekFrontItemOfQueue

      getAllBlockLogs(self, peakedNextItem, nextBlockGroupedBatchItems)
    } else if arbEventIsInSameBlock {
      // Js.log("is in same block but after")
      popNextItemOnChainQueueAndRecurse()
    } else if arbItemIsEarlier {
      // Js.log("is earier but different block")
      let rec addItemAndCheckNextItemForRecursion = item => {
        let _ = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.pop
        let nextBlockGroupedBatchItems = blockGroupedBatchItems->Belt.Array.concat([item])

        let optPeekedArbItem = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.top

        switch optPeekedArbItem {
        | Some(peekedArbItem) =>
          if (peekedArbItem.chain, peekedArbItem.blockNumber) == (item.chain, item.blockNumber) {
            nextBlockGroupedBatchItems->Belt.Array.concat(
              addItemAndCheckNextItemForRecursion(peekedArbItem),
            )
          } else {
            nextBlockGroupedBatchItems
          }

        | None => nextBlockGroupedBatchItems
        }
      }

      Some(addItemAndCheckNextItemForRecursion(peekedArbItem))
    } else {
      // Js.log("after but different block")
      popNextItemOnChainQueueAndRecurse()
    }
  }
}
let popBlockBatchItems = (self: t): option<blockGroupedBatchItems> => {
  //Peek all next fetched event queue items on all chain fetchers
  let peekChainFetcherFrontItems =
    self.chainFetchers
    ->ChainMap.values
    ->Belt.Array.map(fetcher => fetcher->ChainFetcher.peekFrontItemOfQueue)
  // Js.log({
  //   "topOfEachQueue": peekChainFetcherFrontItems->Belt.Array.map(a =>
  //     switch a {
  //     | ChainFetcher.Item(batchItem) => batchItem.event
  //     | ChainFetcher.NoItem(_, chainId) => `NO ITEM ${chainId->Obj.magic}`->Obj.magic
  //     }
  //   ),
  // })

  //Compare the peeked items and determine the next item
  let nextItemFromBuffer = peekChainFetcherFrontItems->determineNextEvent->Belt.Result.getExn
  // Js.log({
  //   "next buffer item": switch nextItemFromBuffer {
  //   | ChainFetcher.Item(batchItem) => batchItem.event
  //   | ChainFetcher.NoItem(_, chainId) => `NO ITEM ${chainId->Obj.magic}`->Obj.magic
  //   },
  // })

  getAllBlockLogs(self, nextItemFromBuffer, [])
}
let rec popBlockBatchAndAwaitItems = async (self: t): option<blockGroupedBatchItems> => {
  //Peek all next fetched event queue items on all chain fetchers
  let peekChainFetcherFrontItems =
    self.chainFetchers
    ->ChainMap.values
    ->Belt.Array.map(fetcher => fetcher->ChainFetcher.peekFrontItemOfQueue)
  // Js.log({
  //   "topOfEachQueue": peekChainFetcherFrontItems->Belt.Array.map(a =>
  //     switch a {
  //     | ChainFetcher.Item(batchItem) => batchItem.event
  //     | ChainFetcher.NoItem(_, chainId) => `NO ITEM ${chainId->Obj.magic}`->Obj.magic
  //     }
  //   ),
  // })

  //Compare the peeked items and determine the next item
  let nextItemFromBuffer = peekChainFetcherFrontItems->determineNextEvent->Belt.Result.getExn
  // Js.log({
  //   "next buffer item": switch nextItemFromBuffer {
  //   | ChainFetcher.Item(batchItem) => batchItem.event
  //   | ChainFetcher.NoItem(_, chainId) => `NO ITEM ${chainId->Obj.magic}`->Obj.magic
  //   },
  // })

  switch nextItemFromBuffer {
  | ChainFetcher.NoItem(latestFullyFetchedBlockTimestampAcrossAllChains, chain) =>
    //Peek arbitraty events queue
    let peekedArbTopItem = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.top

    switch peekedArbTopItem {
    | Some(peekedArbItem)
      // If there is item on the arbitray events queue AND it is a lower timestamp that the lowest timestamp of any chain queue then pop it for the batch
      // the chain fetcher queue
      if peekedArbItem.timestamp <= latestFullyFetchedBlockTimestampAcrossAllChains =>
      getAllBlockLogs(self, nextItemFromBuffer, [])
    | Some(_) // If there are arbitrary items but they are after the lowest chain fetcher latest fetched timestemp we continue to wait.
    | None =>
      //If higest priority is a "NoItem", it means we need to wait for
      //that chain fetcher to fetch blocks of a higher timestamp
      let fetcher = self->getChainFetcher(~chain)
      //Add a callback and wait for a new block range to finish being queried
      await fetcher->ChainFetcher.addNewRangeQueriedCallback
      //Once there is confirmation from the chain fetcher that a new range has been
      //queried retry the popAwait batch function
      await self->popBlockBatchAndAwaitItems
    }
  | ChainFetcher.Item(_batchItem) => getAllBlockLogs(self, nextItemFromBuffer, [])
  }
}
//TODO: investigate combining logic with the above synchronus version of this function

/**
Async pop function that will wait for an item to be available before returning
*/
let rec popAndAwaitBatchItem: t => promise<Types.eventBatchQueueItem> = async (
  self: t,
): Types.eventBatchQueueItem => {
  //Peek all next fetched event queue items on all chain fetchers
  let peekChainFetcherFrontItems =
    self.chainFetchers
    ->ChainMap.values
    ->Belt.Array.map(fetcher => fetcher->ChainFetcher.peekFrontItemOfQueue)

  //Compare the peeked items and determine the next item
  let nextItemFromBuffer = peekChainFetcherFrontItems->determineNextEvent->Belt.Result.getExn

  //Callback for handling popping of chain fetcher events
  let popNextItemAndAwait = async () => {
    switch nextItemFromBuffer {
    | ChainFetcher.NoItem(_, chain) =>
      //If higest priority is a "NoItem", it means we need to wait for
      //that chain fetcher to fetch blocks of a higher timestamp
      let fetcher = self->getChainFetcher(~chain)
      //Add a callback and wait for a new block range to finish being queried
      await fetcher->ChainFetcher.addNewRangeQueriedCallback
      //Once there is confirmation from the chain fetcher that a new range has been
      //queried retry the popAwait batch function
      await self->popAndAwaitBatchItem
    | ChainFetcher.Item(batchItem) =>
      //If there is an item pop it off of the chain fetcher queue and return
      let fetcher = self->getChainFetcher(~chain=batchItem.chain)
      await fetcher->ChainFetcher.popAndAwaitQueueItem
    }
  }

  //Peek arbitraty events queue
  let peekedArbTopItem = self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.top

  switch peekedArbTopItem {
  //If there is item on the arbitray events queue, pop the relevant item from
  //the chain fetcher queue
  | None => await popNextItemAndAwait()
  | Some(peekedArbItem) =>
    //If there is an item on the arbitrary events queue, compare it to the next
    //item from the chain fetchers
    let arbItemIsEarlier = chainFetcherPeekComparitorEarliestEvent(
      ChainFetcher.Item(peekedArbItem),
      nextItemFromBuffer,
    )

    //If the arbitrary item is earlier, return that
    if arbItemIsEarlier {
      //safely pop the item since we have already checked there's one at the front
      self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.pop->Belt.Option.getUnsafe
    } else {
      //Else pop the next item from chain fetchers
      await popNextItemAndAwait()
    }
  }
}

let createBatch = async (self: t, ~minBatchSize: int, ~maxBatchSize: int): array<
  Types.eventBatchQueueItem,
> => {
  let refTime = Hrtime.makeTimer()

  let batch = []
  while batch->Belt.Array.length < minBatchSize {
    let item = await self->popAndAwaitBatchItem
    batch->Js.Array2.push(item)->ignore
  }

  let moreItemsToPop = ref(true)
  while moreItemsToPop.contents && batch->Belt.Array.length < maxBatchSize {
    let optItem = self->popBatchItem
    switch optItem {
    | None => moreItemsToPop := false
    | Some(item) => batch->Js.Array2.push(item)->ignore
    }
  }
  let fetchedEventsBuffer =
    self.chainFetchers
    ->ChainMap.values
    ->Belt.Array.map(fetcher => (
      fetcher.chainConfig.chain->ChainMap.Chain.toString,
      fetcher.fetchedEventQueue.queue->SDSL.Queue.size,
    ))
    ->Belt.Array.concat([
      ("arbitrary", self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.length),
    ])
    ->Js.Dict.fromArray

  let timeElapsed = refTime->Hrtime.timeSince->Hrtime.toMillis

  Logging.trace({
    "message": "New batch created for processing",
    "batch size": batch->Array.length,
    "buffers": fetchedEventsBuffer,
    "time taken (ms)": timeElapsed,
  })

  batch
}

let addItemToArbitraryEvents = (self: t, item: Types.eventBatchQueueItem) => {
  self.arbitraryEventPriorityQueue->SDSL.PriorityQueue.push(item)->ignore
}

module ExposedForTesting_Hidden = {
  let priorityQueueComparitor = priorityQueueComparitor
  let getComparitorFromItem = getComparitorFromItem
  let createDetermineNextEventFunction = createDetermineNextEventFunction
}
