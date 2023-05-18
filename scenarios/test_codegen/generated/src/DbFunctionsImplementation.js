const postgres = require("postgres")

const postgresConfig = require("./Config.bs.js").db

const sql = postgres({...postgresConfig, 
  transform: {
    undefined: null
  }
})


// db operations for raw_events:

module.exports.readRawEventsEntities = (entityIdArray) => sql`
  SELECT *
  FROM public.raw_events
  WHERE (chain_id, event_id) IN ${sql(entityIdArray)}`;

module.exports.batchSetRawEvents = (entityDataArray) => {
  const valueCopyToFixBigIntType = entityDataArray; // This is required for BigInts to work in the db. See: https://github.com/Float-Capital/indexer/issues/212
  return sql`
    INSERT INTO public.raw_events
  ${sql(
    valueCopyToFixBigIntType,
    "chain_id",
    "event_id",
    "block_number",
    "log_index",
    "transaction_index",
    "transaction_hash",
    "src_address",
    "block_hash",
    "block_timestamp",
    "event_type",
    "params"
  )}
    ON CONFLICT(chain_id, event_id) DO UPDATE
    SET
    "chain_id" = EXCLUDED."chain_id",
    "event_id" = EXCLUDED."event_id",
    "block_number" = EXCLUDED."block_number",
    "log_index" = EXCLUDED."log_index",
    "transaction_index" = EXCLUDED."transaction_index",
    "transaction_hash" = EXCLUDED."transaction_hash",
    "src_address" = EXCLUDED."src_address",
    "block_hash" = EXCLUDED."block_hash",
    "block_timestamp" = EXCLUDED."block_timestamp",
    "event_type" = EXCLUDED."event_type",
    "params" = EXCLUDED."params"
  ;`;
};

module.exports.batchDeleteRawEvents = (entityIdArray) => sql`
  DELETE
  FROM public.raw_events
  WHERE (chain_id, event_id) IN ${sql(entityIdArray)};`;
// end db operations for raw_events

  // db operations for User:

  module.exports.readUserEntities = (entityIdArray) => sql`
  SELECT *
  FROM public.user
  WHERE id IN ${sql(entityIdArray)}`

  module.exports.batchSetUser = (entityDataArray) => {
  const valueCopyToFixBigIntType = entityDataArray // This is required for BigInts to work in the db. See: https://github.com/Float-Capital/indexer/issues/212
  return sql`
    INSERT INTO public.user
  ${sql(valueCopyToFixBigIntType,
    "id",
    "address",
    "gravatar",
  )}
    ON CONFLICT(id) DO UPDATE
    SET
    "id" = EXCLUDED."id",
    "address" = EXCLUDED."address",
    "gravatar" = EXCLUDED."gravatar"
  ;`
  }

  module.exports.batchDeleteUser = (entityIdArray) => sql`
  DELETE
  FROM public.user
  WHERE id IN ${sql(entityIdArray)};`
  // end db operations for User

  // db operations for Gravatar:

  module.exports.readGravatarEntities = (entityIdArray) => sql`
  SELECT *
  FROM public.gravatar
  WHERE id IN ${sql(entityIdArray)}`

  module.exports.batchSetGravatar = (entityDataArray) => {
  const valueCopyToFixBigIntType = entityDataArray // This is required for BigInts to work in the db. See: https://github.com/Float-Capital/indexer/issues/212
  return sql`
    INSERT INTO public.gravatar
  ${sql(valueCopyToFixBigIntType,
    "id",
    "owner",
    "displayName",
    "imageUrl",
    "updatesCount",
  )}
    ON CONFLICT(id) DO UPDATE
    SET
    "id" = EXCLUDED."id",
    "owner" = EXCLUDED."owner",
    "displayName" = EXCLUDED."displayName",
    "imageUrl" = EXCLUDED."imageUrl",
    "updatesCount" = EXCLUDED."updatesCount"
  ;`
  }

  module.exports.batchDeleteGravatar = (entityIdArray) => sql`
  DELETE
  FROM public.gravatar
  WHERE id IN ${sql(entityIdArray)};`
  // end db operations for Gravatar

