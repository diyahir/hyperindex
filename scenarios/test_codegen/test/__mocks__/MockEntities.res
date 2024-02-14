let gravatarEntity1: Types.gravatarEntity = {
  id: "1001",
  owner_id: "0x123",
  displayName: "gravatar1",
  imageUrl: "https://gravatar1.com",
  updatesCount: Ethers.BigInt.fromInt(0),
}

let gravatarEntity2: Types.gravatarEntity = {
  id: "1002",
  owner_id: "0x678",
  displayName: "gravatar2",
  imageUrl: "https://gravatar2.com",
  updatesCount: Ethers.BigInt.fromInt(1),
}

let gravatarSerialized1 = gravatarEntity1->Types.gravatarEntity_encode
let gravatarSerialized2 = gravatarEntity2->Types.gravatarEntity_encode
let mockInMemRow1: Types.inMemoryStoreRow<Js.Json.t> = {
  entity: gravatarSerialized1,
  dbOp: Types.Set,
}

let mockInMemRow2: Types.inMemoryStoreRow<Js.Json.t> = {
  entity: gravatarSerialized2,
  dbOp: Types.Set,
}
