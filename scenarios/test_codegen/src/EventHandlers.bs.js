// Generated by ReScript, PLEASE EDIT WITH CARE
'use strict';

var Curry = require("rescript/lib/js/curry.js");
var Ethers = require("generated/src/bindings/Ethers.bs.js");
var Handlers = require("generated/src/Handlers.bs.js");
var Belt_Option = require("rescript/lib/js/belt_Option.js");

Handlers.GravatarContract.registerNewGravatarLoadEntities(function (param, param$1) {
      
    });

Handlers.GravatarContract.registerNewGravatarHandler(function ($$event, context) {
      Curry._1(context.gravatar.insert, {
            id: $$event.params.id.toString(),
            owner: Ethers.ethAddressToString($$event.params.owner),
            ownerData: undefined,
            displayName: $$event.params.displayName,
            imageUrl: $$event.params.imageUrl,
            updatesCount: BigInt(1)
          });
    });

Handlers.GravatarContract.registerUpdatedGravatarLoadEntities(function ($$event, context) {
      var gravatarLoader = Curry._1(context.gravatar.gravatarWithChangesLoad, $$event.params.id.toString());
      Curry._1(gravatarLoader.ownerLoad, undefined);
    });

Handlers.GravatarContract.registerUpdatedGravatarHandler(function ($$event, context) {
      var updatesCount = Belt_Option.mapWithDefault(Curry._1(context.gravatar.gravatarWithChanges, undefined), BigInt(1), (function (gravatar) {
              return Ethers.$$BigInt.add(gravatar.updatesCount, BigInt(1));
            }));
      Curry._1(context.gravatar.update, {
            id: $$event.params.id.toString(),
            owner: Ethers.ethAddressToString($$event.params.owner),
            ownerData: undefined,
            displayName: $$event.params.displayName,
            imageUrl: $$event.params.imageUrl,
            updatesCount: updatesCount
          });
    });

/*  Not a pure module */
