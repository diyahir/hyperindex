// TODO - put common bindings into bindings folder:
module Postgres = {
  type sql

  type poolConfig = {
    host: string,
    port: int,
    user: string,
    password: string,
    database: string,
  }

  @module
  external makeSql: (~config: poolConfig) => sql = "postgres"

  // TODO: can explore this approach (https://forum.rescript-lang.org/t/rfc-support-for-tagged-template-literals/3744)
  // @send @variadic
  // external sql:  array<string>  => (sql, array<string>) => int = "sql"
}

let sql = Postgres.makeSql(~config=Config.db->Obj.magic /* TODO: make this have the correct type */)

module User = {
  let createUserTable: unit => promise<unit> = async () => {
    await %raw(
      "sql`CREATE TABLE public.user (id text  NOT NULL,address text  NOT NULL,gravatar text,UNIQUE (id));`"
    )
  }

  let deleteUserTable: unit => promise<unit> = async () => {
    // NOTE: we can refine the `IF EXISTS` part because this now prints to the terminal if the table doesn't exist (which isn't nice for the developer).
    await %raw("sql`DROP TABLE IF EXISTS public.user;`")
  }
}

module Gravatar = {
  let createGravatarTable: unit => promise<unit> = async () => {
    await %raw(
      "sql`CREATE TABLE public.gravatar (id text  NOT NULL,owner text  NOT NULL,displayName text  NOT NULL,imageUrl text  NOT NULL,updatesCount integer  NOT NULL,UNIQUE (id));`"
    )
  }

  let deleteGravatarTable: unit => promise<unit> = async () => {
    // NOTE: we can refine the `IF EXISTS` part because this now prints to the terminal if the table doesn't exist (which isn't nice for the developer).
    await %raw("sql`DROP TABLE IF EXISTS public.gravatar;`")
  }
}

type t
@module external process: t = "process"

@send external exit: (t, unit) => unit = "exit"

// TODO: all the migration steps should run as a single transaction
let runUpMigrations = async () => {
  // TODO: catch and handle query errors
  await User.createUserTable()
  await Gravatar.createGravatarTable()
}

let runDownMigrations = async () => {
  await User.deleteUserTable()
  await Gravatar.deleteGravatarTable()
}

let setupDb = async () => {
  // TODO: we should make a hash of the schema file (that gets stored in the DB) and either drop the tables and create new ones or keep this migration.
  //       for now we always run the down migration.
  // if (process.env.MIGRATE === "force" || hash_of_schema_file !== hash_of_current_schema)
  await runDownMigrations()
  // else
  //   await clearDb()

  await runUpMigrations()

  process->exit()
}
