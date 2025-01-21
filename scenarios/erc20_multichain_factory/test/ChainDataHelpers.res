open Belt

let getDefaultAddress = (chain, contractName) => {
  let chainConfig = RegisterHandlers.getConfig().chainMap->ChainMap.get(chain)
  let contract = chainConfig.contracts->Js.Array2.find(c => c.name == contractName)->Option.getExn
  let defaultAddress = contract.addresses[0]->Option.getExn
  defaultAddress
}

let gAS_USED_DEFAULT = BigInt.zero
let makeBlock = (~blockNumber, ~blockTimestamp, ~blockHash) =>
  {
    number: blockNumber,
    hash: blockHash,
    timestamp: blockTimestamp,
    gasUsed: gAS_USED_DEFAULT,
  }->(Utils.magic: Types.Block.t => Internal.eventBlock)

let makeTransaction = (~transactionIndex, ~transactionHash) =>
  {
    transactionIndex,
    hash: transactionHash,
  }->(Utils.magic: Types.Transaction.t => Internal.eventTransaction)

module ERC20 = {
  let contractName = "ERC20"
  let getDefaultAddress = getDefaultAddress(_, contractName)
  module Transfer = {
    let mkEventConstrWithParamsAndAddress =
      MockChainData.makeEventConstructor(
        ~eventMod=module(Types.ERC20.Transfer),
        ~makeBlock,
        ~makeTransaction,
        ...
      )

    let mkEventConstr = (params, ~chain) =>
      mkEventConstrWithParamsAndAddress(~srcAddress=getDefaultAddress(chain), ~params, ...)
  }
}

module ERC20Factory = {
  let contractName = "ERC20Factory"
  let getDefaultAddress = getDefaultAddress(_, contractName)

  module TokenCreated = {
    let mkEventConstrWithParamsAndAddress =
      MockChainData.makeEventConstructor(
        ~eventMod=module(Types.ERC20Factory.TokenCreated),
        ~makeBlock,
        ~makeTransaction,
        ...
      )

    let mkEventConstr = (params, ~chain) =>
      mkEventConstrWithParamsAndAddress(~srcAddress=getDefaultAddress(chain), ~params, ...)
  }
  module DeleteUser = {
    let mkEventConstrWithParamsAndAddress =
      MockChainData.makeEventConstructor(
        ~eventMod=module(Types.ERC20Factory.DeleteUser),
        ~makeBlock,
        ~makeTransaction,
        ...
      )

    let mkEventConstr = (params, ~chain) =>
      mkEventConstrWithParamsAndAddress(~srcAddress=getDefaultAddress(chain), ~params, ...)
  }
}

module Stubs = {
  type t = {
    mockChainDataMap: ChainMap.t<MockChainData.t>,
    tasks: ref<array<GlobalState.task>>,
    gsManager: GlobalStateManager.t,
  }

  let make = (~mockChainDataMap, ~tasks, ~gsManager) => {
    mockChainDataMap,
    tasks,
    gsManager,
  }
  let getTasks = ({tasks}) => tasks.contents
  let getMockChainData = ({mockChainDataMap}, chain) => mockChainDataMap->ChainMap.get(chain)

  //Stub executePartitionQuery with mock data
  let makeExecutePartitionQuery = (stubData: t) => async (
    query,
    ~logger,
    ~source,
    ~currentBlockHeight,
    ~chain,
  ) => {
    (logger, currentBlockHeight, source)->ignore
    stubData->getMockChainData(chain)->MockChainData.executeQuery(query)->Ok
  }

  //Stub for getting block hashes instead of the worker
  let makeGetBlockHashes = (~stubData, ~source) => async (~blockNumbers, ~logger as _) => {
    let module(Source: Source.S) = source
    stubData->getMockChainData(Source.chain)->MockChainData.getBlockHashes(~blockNumbers)->Ok
  }

  let replaceNexQueryCheckAllChainsWithGivenChain = ({tasks}: t, chain) => {
    tasks :=
      tasks.contents->Array.map(t =>
        switch t {
        | GlobalState.NextQuery(CheckAllChains) => GlobalState.NextQuery(Chain(chain))
        | task => task
        }
      )
  }

  //Stub wait for new block
  let makeWaitForNewBlock = (stubData: t) => async (source, ~currentBlockHeight, ~logger) => {
    (logger, currentBlockHeight)->ignore
    let module(Source: Source.S) = source
    stubData->getMockChainData(Source.chain)->MockChainData.getHeight
  }
  //Stub dispatch action to set state and not dispatch task but store in
  //the tasks ref
  let makeDispatchAction = ({gsManager, tasks}, action) => {
    let (nextState, nextTasks) = GlobalState.actionReducer(
      gsManager->GlobalStateManager.getState,
      action,
    )
    gsManager->GlobalStateManager.setState(nextState)
    tasks := tasks.contents->Array.concat(nextTasks)
  }

  let makeDispatchTask = (stubData: t, task) => {
    GlobalState.injectedTaskReducer(
      ~executeQuery=makeExecutePartitionQuery(stubData),
      ~waitForNewBlock=makeWaitForNewBlock(stubData),
      ~rollbackLastBlockHashesToReorgLocation=chainFetcher =>
        chainFetcher->ChainFetcher.rollbackLastBlockHashesToReorgLocation(
          ~getBlockHashes=makeGetBlockHashes(~stubData, ~source=chainFetcher.chainConfig.source),
        ),
    )(
      ~dispatchAction=makeDispatchAction(stubData, _),
      stubData.gsManager->GlobalStateManager.getState,
      task,
    )
  }

  let dispatchAllTasks = async (stubData: t) => {
    let tasksToRun = stubData.tasks.contents
    stubData.tasks := []
    let _ =
      await tasksToRun
      ->Array.map(task => makeDispatchTask(stubData, task))
      ->Js.Promise.all
  }
}
