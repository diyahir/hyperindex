// Generated by ReScript, PLEASE EDIT WITH CARE
'use strict';

var IO = require("generated/src/IO.bs.js");
var Jest = require("@glennsl/rescript-jest/src/jest.bs.js");
var DbStub = require("./__mocks__/DbStub.bs.js");
var Context = require("generated/src/Context.bs.js");
var Js_dict = require("rescript/lib/js/js_dict.js");
var MockEvents = require("./__mocks__/MockEvents.bs.js");
var ContextMock = require("./__mocks__/ContextMock.bs.js");
var DbFunctions = require("generated/src/DbFunctions.bs.js");
var MockEntities = require("./__mocks__/MockEntities.bs.js");
var EventProcessing = require("generated/src/EventProcessing.bs.js");

Jest.describe("E2E Mock Event Batch", (function (param) {
        Jest.beforeAllPromise(undefined, (async function (param) {
                DbStub.setGravatarDb(MockEntities.gravatarEntity1);
                DbStub.setGravatarDb(MockEntities.gravatarEntity2);
                return await EventProcessing.processEventBatch(MockEvents.eventBatch, ContextMock.mockContext);
              }));
        afterAll(function () {
              ContextMock.insertMock.mockClear();
              ContextMock.updateMock.mockClear();
            });
        Jest.test("3 newgravatar event insert calls in order", (function (param) {
                var insertCalls = Jest.MockJs.calls(ContextMock.insertMock);
                return Jest.Expect.toEqual(Jest.Expect.expect(insertCalls), [
                            MockEvents.newGravatar1.id,
                            MockEvents.newGravatar2.id,
                            MockEvents.newGravatar3.id
                          ]);
              }));
        Jest.test("3 updategravatar event insert calls in order", (function (param) {
                var insertCalls = Jest.MockJs.calls(ContextMock.insertMock);
                return Jest.Expect.toEqual(Jest.Expect.expect(insertCalls), [
                            MockEvents.updatedGravatar1.id,
                            MockEvents.updatedGravatar2.id,
                            MockEvents.updatedGravatar3.id
                          ]);
              }));
      }));

Jest.describe("E2E Db check", (function (param) {
        Jest.beforeAllPromise(undefined, (async function (param) {
                await DbFunctions.batchSetGravatar([
                      MockEntities.gravatarEntity1,
                      MockEntities.gravatarEntity2
                    ]);
                return await EventProcessing.processEventBatch(MockEvents.eventBatch, Context.getContext(undefined));
              }));
        Jest.test("Validate inmemory store state", (function (param) {
                var inMemoryStore = IO.InMemoryStore.gravatarDict.contents;
                var inMemoryStoreRows = Js_dict.values(inMemoryStore);
                return Jest.Expect.toEqual(Jest.Expect.expect(inMemoryStoreRows), [
                            {
                              crud: /* Update */2,
                              entity: {
                                id: "1001",
                                owner: "0x123",
                                displayName: "update1",
                                imageUrl: "https://gravatar1.com",
                                updatesCount: 2
                              }
                            },
                            {
                              crud: /* Update */2,
                              entity: {
                                id: "1002",
                                owner: "0x456",
                                displayName: "update2",
                                imageUrl: "https://gravatar2.com",
                                updatesCount: 2
                              }
                            },
                            {
                              crud: /* Create */0,
                              entity: {
                                id: "1003",
                                owner: "0x789",
                                displayName: "update3",
                                imageUrl: "https://gravatar3.com",
                                updatesCount: 2
                              }
                            }
                          ]);
              }));
      }));

/*  Not a pure module */
