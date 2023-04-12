module Schema = {
  type table<'rowType> = 'rowType

  type fieldSelector

  @module("drizzle-orm/pg-core")
  external pgTable: (~name: string, ~fields: 'fields) => table<'rowType> = "pgTable"

  type field

  @module("drizzle-orm/pg-core")
  external serial: string => field = "serial"
  @module("drizzle-orm/pg-core")
  external text: string => field = "text"
  @module("drizzle-orm/pg-core")
  external integer: string => field = "integer"
  @module("drizzle-orm/pg-core")
  external numeric: string => field = "numeric"
  @module("drizzle-orm/pg-core")
  external boolean: string => field = "boolean"
  @module("drizzle-orm/pg-core")
  external json: string => field = "json"
  @module("drizzle-orm/pg-core")
  external jsonb: string => field = "jsonb"
  @module("drizzle-orm/pg-core")
  external time: string => field = "time"
  @module("drizzle-orm/pg-core")
  external timestamp: string => field = "timestamp"
  @module("drizzle-orm/pg-core")
  external date: string => field = "date"
  @module("drizzle-orm/pg-core")
  external varchar: string => field = "varchar"

  @send
  external primaryKey: field => field = "primaryKey"
}

module Pool = {
  type t

  type poolConfig = {
    host: string,
    port: int,
    user: string,
    password: string,
    database: string,
  }

  @module("pg") @new
  external make: (~config: poolConfig) => t = "Pool"
}

module Drizzle = {
  type db

  //TODO: If we use any other methods on drizzle perhap have a drizzle
  //type with send methods
  @module("drizzle-orm/node-postgres")
  external make: (~pool: Pool.t) => db = "drizzle"

  type insertion

  @send
  external insert: (db, ~table: Schema.table<'a>) => insertion = "insert"

  type deletion
  type crudOperation<'a>
  @send
  external delete: (db, ~table: Schema.table<'a>) => crudOperation<deletion> = "delete"

  type whereSelector
  @module("drizzle-orm/expressions")
  external eq: (~field: Schema.fieldSelector, ~value: 'a) => whereSelector = "eq"

  @send
  external where: (crudOperation<'a>, ~condition: 'condition) => promise<'b> = "where"

  type selection
  @send external select: db => crudOperation<selection> = "select"

  @send external from: (crudOperation<'a>, ~table: Schema.table<'b>) => crudOperation<'a> = "from"

  type migrationsConfig = {migrationsFolder: string}
  @module("drizzle-orm/node-postgres/migrator")
  external migrate: (db, migrationsConfig) => promise<unit> = "migrate"

  type returnedValues<'a> = 'a
  type values<'a, 'b> = (insertion, 'a) => returnedValues<'b>
  @send
  external values: (insertion, 'a) => returnedValues<'b> = "values"

  type targetConflict<'conflictId, 'valuesToSet> = {
    target: 'conflictId,
    set?: 'valuesToSet,
  }

  type dbReturn = unit // unit until we care about this

  @send
  external onConflictDoUpdate: (returnedValues<'a>, targetConflict<'b, 'c>) => promise<dbReturn> =
    "onConflictDoUpdate"

  @send
  external onConflictDoNothing: (returnedValues<'a>, targetConflict<'b, 'c>) => promise<dbReturn> =
    "onConflictDoNothing"
}
