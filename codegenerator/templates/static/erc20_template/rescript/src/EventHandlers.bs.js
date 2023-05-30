// Generated by ReScript, PLEASE EDIT WITH CARE
'use strict';

var Curry = require("rescript/lib/js/curry.js");
var Handlers = require("generated/src/Handlers.bs.js");

Handlers.ERC20Contract.registerCreationLoadEntities(function ($$event, context) {
      Curry._1(context.tokens.tokensCreationLoad, $$event.srcAddress);
    });

Handlers.ERC20Contract.registerCreationHandler(function ($$event, context) {
      Curry._1(context.tokens.insert, {
            id: $$event.srcAddress,
            name: $$event.params.name,
            symbol: $$event.params.symbol,
            decimals: 18
          });
    });

/*  Not a pure module */
