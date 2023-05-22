type chainId = int
type eventId = Ethers.BigInt.t

module RawEvents = {
  type rawEventRowId = (chainId, eventId)
  @module("./DbFunctionsImplementation.js")
  external batchSetRawEvents: array<Types.rawEventsEntity> => promise<unit> = "batchSetRawEvents"

  @module("./DbFunctionsImplementation.js")
  external batchDeleteRawEvents: array<rawEventRowId> => promise<unit> = "batchDeleteRawEvents"

  @module("./DbFunctionsImplementation.js")
  external readRawEventsEntities: array<rawEventRowId> => promise<array<Types.rawEventsEntity>> =
    "readRawEventsEntities"
}
  // {
  //   crud: 0,
  //   entity: {
  //     id: '1003',
  //     owner: '0x7890000000000000000000000000000000000000',
  //     displayName: 'update3',
  //     imageUrl: 'https://gravatar3.com',
  //     updatesCount: 2
  //   },
  //   eventData: { event_chain_id: 1337, event_id: 65537n }
  // },
  // {
  //   crud: 1,
  //   entity: '(1002,0x678,gravatar2,https://gravatar2.com,1)',
  //   eventData: undefined
  // }


type readEntityData<'a> = {
   entity: 'a,
   eventData: Types.eventData
}

{{#each entities as |entity|}}
module {{entity.name.capitalized}} = {
  type {{entity.name.uncapitalized}}ReadRow = {
  {{#each entity.params as |param|}}
     {{param.key}}: {{param.type_rescript}}, 
  {{/each}}
  @as("event_chain_id") chainId: int,
  @as("event_id") eventId: Ethers.BigInt.t,
  }

  let readRowToReadEntityData = (readRow: {{entity.name.uncapitalized}}ReadRow): readEntityData<Types.{{entity.name.uncapitalized}}Entity> => {
    let {
      {{#each entity.params as | param |}}
      {{param.key}},
      {{/each}}
      chainId,
      eventId
    } = readRow

    {
      entity: {
        {{#each entity.params as | param |}}
        {{param.key}},
        {{/each}}
      },
      eventData: {
        chainId,
        eventId
      }
    }
  }

  @module("./DbFunctionsImplementation.js")
  external batchSet{{entity.name.capitalized}}: array<Types.inMemoryStoreRow<Types.{{entity.name.uncapitalized}}Entity>> => promise<(unit)> = "batchSet{{entity.name.capitalized}}"

  @module("./DbFunctionsImplementation.js")
  external batchDelete{{entity.name.capitalized}}: array<Types.id> => promise<(unit)> = "batchDelete{{entity.name.capitalized}}"

  @module("./DbFunctionsImplementation.js")
  external read{{entity.name.capitalized}}Entities: array<Types.id> => promise<array<{{entity.name.uncapitalized}}ReadRow>> = "read{{entity.name.capitalized}}Entities"

  // let read{{entity.name.capitalized}}Entities: array<Types.id> => promise<array<readEntityEventData<Types.{{entity.name.uncapitalized}}Entity>>> = async (idArr) => {
  // let res = await idArr->read{{entity.name.capitalized}}EntitiesUnclen
  // res->Belt.Array.map(uncleanItem => uncleanItem->readEntityDataToInMemRow(~entityConverter=readTypeToInMemRow))
  // }
}
{{/each}}
