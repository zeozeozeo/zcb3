import { bundle_and_download } from './snippets/zcb3-95e453e030ce1483/inline0.js';

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_a86c216575a75c30: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_dd5d2d07ce9e6c57: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_4bd7a57e54337366: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_function_49868bde5eb1e745: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_40c5a80572e8f9d3: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_c0cca72b82b86f4d: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_7579aab02a8a620c: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_914df97fcfa788f2: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_81fc77679af83bc6: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_3c3b4f651835fbcb: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_abort_5ee4083ce26e0b01: function(arg0) {
            arg0.abort();
        },
        __wbg_abort_7a67cb8f9383baa1: function(arg0, arg1) {
            arg0.abort(arg1);
        },
        __wbg_activeElement_41dff9147c0c1503: function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_activeElement_80ade59832af5117: function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_activeTexture_55755e76627be758: function(arg0, arg1) {
            arg0.activeTexture(arg1 >>> 0);
        },
        __wbg_activeTexture_bec0539b102730b3: function(arg0, arg1) {
            arg0.activeTexture(arg1 >>> 0);
        },
        __wbg_addEventListener_4696109b6f15c412: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_addEventListener_83ef16da0995f634: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_altKey_7a24c21194788eb1: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_altKey_dac3f7f22baf3c82: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_appendChild_8eab65de52dd0834: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_append_c015600138ae60bb: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_arrayBuffer_7bba74066875530e: function(arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        },
        __wbg_arrayBuffer_dae084a298aa5fe0: function() { return handleError(function (arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        }, arguments); },
        __wbg_at_16485bb9c4fe6761: function(arg0, arg1) {
            const ret = arg0.at(arg1);
            return ret;
        },
        __wbg_attachShader_73ba3bb26991b2f3: function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        },
        __wbg_attachShader_91626cdf6ee920b8: function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        },
        __wbg_bindBuffer_da48260900fd87cb: function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindBuffer_ec76634c95f563c2: function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindTexture_3f1c468809dfc331: function(arg0, arg1, arg2) {
            arg0.bindTexture(arg1 >>> 0, arg2);
        },
        __wbg_bindTexture_82948e04f9a38b3e: function(arg0, arg1, arg2) {
            arg0.bindTexture(arg1 >>> 0, arg2);
        },
        __wbg_bindVertexArrayOES_e9c08ca73f91231f: function(arg0, arg1) {
            arg0.bindVertexArrayOES(arg1);
        },
        __wbg_bindVertexArray_ef65b171588388e0: function(arg0, arg1) {
            arg0.bindVertexArray(arg1);
        },
        __wbg_blendEquationSeparate_91ba074ad013b85b: function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blendEquationSeparate_faa06617b84f5c1f: function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blendFuncSeparate_9de3db6383af1e0c: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_blendFuncSeparate_fb17a9951727aac3: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_blockSize_e20f2753687379d2: function(arg0) {
            const ret = arg0.blockSize;
            return ret;
        },
        __wbg_blur_2a28d7189bd9dbc7: function() { return handleError(function (arg0) {
            arg0.blur();
        }, arguments); },
        __wbg_body_401b41698e8b50fe: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_bottom_6dbc1e08a40f8ad4: function(arg0) {
            const ret = arg0.bottom;
            return ret;
        },
        __wbg_bufferData_143a9bcd4d03d07c: function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_bufferData_74194b1c2d90193e: function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_buffer_a77cc90da4bdb503: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_bundle_and_download_f129f3958b76d76b: function() { return handleError(function () {
            const ret = bundle_and_download();
            return ret;
        }, arguments); },
        __wbg_button_225d9d40d1b0539a: function(arg0) {
            const ret = arg0.button;
            return ret;
        },
        __wbg_call_7f2987183bb62793: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_d578befcc3145dee: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_cancelAnimationFrame_19ab829762998ae9: function() { return handleError(function (arg0, arg1) {
            arg0.cancelAnimationFrame(arg1);
        }, arguments); },
        __wbg_changedTouches_78d90c47b062c222: function(arg0) {
            const ret = arg0.changedTouches;
            return ret;
        },
        __wbg_clearColor_4c23ac0feeb06fb4: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        },
        __wbg_clearColor_b8afe0751e9d7a54: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        },
        __wbg_clearInterval_c7f2b17b895f0bc4: function(arg0, arg1) {
            arg0.clearInterval(arg1);
        },
        __wbg_clearTimeout_113b1cde814ec762: function(arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        },
        __wbg_clearTimeout_6b8d9a38b9263d65: function(arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        },
        __wbg_clear_4d247257533aabcb: function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        },
        __wbg_clear_98a9ca84e00ae8e2: function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        },
        __wbg_click_71fd1ebdfba801e9: function(arg0) {
            arg0.click();
        },
        __wbg_clientX_55a6e886f99430c0: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientX_dbc3f9f00736bbd4: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientY_010f69b0d7c9cf50: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_clientY_6a206add565bb2a7: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_clipboardData_e9e89b525c2a07ab: function(arg0) {
            const ret = arg0.clipboardData;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_clipboard_949fe156c7f0644e: function(arg0) {
            const ret = arg0.clipboard;
            return ret;
        },
        __wbg_colorMask_134144611b082d70: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        },
        __wbg_colorMask_67f0083d53f15052: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        },
        __wbg_compileShader_30b1185156c62e3a: function(arg0, arg1) {
            arg0.compileShader(arg1);
        },
        __wbg_compileShader_d097925490ad9cba: function(arg0, arg1) {
            arg0.compileShader(arg1);
        },
        __wbg_contentBoxSize_eeeb1078851bcdc4: function(arg0) {
            const ret = arg0.contentBoxSize;
            return ret;
        },
        __wbg_contentRect_ffc7f5bc1857d6fe: function(arg0) {
            const ret = arg0.contentRect;
            return ret;
        },
        __wbg_createBuffer_8dc942ca97cf9d2a: function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createBuffer_bdda716ebf68ba59: function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createElement_8640e331213b402e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createObjectURL_470fa06cc4a9e8f0: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_createProgram_03cf82c6259699da: function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createProgram_ba013605ddf3824a: function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createShader_b2c5333fcc05114e: function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createShader_f556b21db86193fd: function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createTexture_ab0a6dde87005cb1: function(arg0) {
            const ret = arg0.createTexture();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createTexture_b2dbf72113bdda56: function(arg0) {
            const ret = arg0.createTexture();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createVertexArrayOES_a89b0d9f1070e733: function(arg0) {
            const ret = arg0.createVertexArrayOES();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createVertexArray_be0c22725872a475: function(arg0) {
            const ret = arg0.createVertexArray();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_ctrlKey_af896fa77d43a375: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_ctrlKey_dc8c7fcd63c26948: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_dataTransfer_994cf664bb1da77f: function(arg0) {
            const ret = arg0.dataTransfer;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_data_31cbb395bfd6c6ce: function(arg0, arg1) {
            const ret = arg1.data;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_debug_50e24f25b064ded1: function(arg0) {
            console.debug(arg0);
        },
        __wbg_deleteBuffer_38cfc45ad05c89ef: function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        },
        __wbg_deleteBuffer_ef356b1392cab959: function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        },
        __wbg_deleteProgram_6eccd9aa110cbb2c: function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        },
        __wbg_deleteProgram_e1eaf172c61bd109: function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        },
        __wbg_deleteShader_0784961238f3ba6f: function(arg0, arg1) {
            arg0.deleteShader(arg1);
        },
        __wbg_deleteShader_13b98e109c7ec22b: function(arg0, arg1) {
            arg0.deleteShader(arg1);
        },
        __wbg_deleteTexture_57bf3a76dc0a7bf9: function(arg0, arg1) {
            arg0.deleteTexture(arg1);
        },
        __wbg_deleteTexture_72eed589178ae2f9: function(arg0, arg1) {
            arg0.deleteTexture(arg1);
        },
        __wbg_deltaMode_389ab9e0c7c47a3c: function(arg0) {
            const ret = arg0.deltaMode;
            return ret;
        },
        __wbg_deltaX_6fd68d53fb18c3ea: function(arg0) {
            const ret = arg0.deltaX;
            return ret;
        },
        __wbg_deltaY_d67fb1a74cff23bc: function(arg0) {
            const ret = arg0.deltaY;
            return ret;
        },
        __wbg_detachShader_584b9575d4695a5a: function(arg0, arg1, arg2) {
            arg0.detachShader(arg1, arg2);
        },
        __wbg_detachShader_e0292cc403c305a7: function(arg0, arg1, arg2) {
            arg0.detachShader(arg1, arg2);
        },
        __wbg_devicePixelContentBoxSize_74f4718d7ccbbda0: function(arg0) {
            const ret = arg0.devicePixelContentBoxSize;
            return ret;
        },
        __wbg_devicePixelRatio_a0dc790882837272: function(arg0) {
            const ret = arg0.devicePixelRatio;
            return ret;
        },
        __wbg_disableVertexAttribArray_502ba5544050cc4a: function(arg0, arg1) {
            arg0.disableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_disableVertexAttribArray_a1f4414d0521b130: function(arg0, arg1) {
            arg0.disableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_disable_5c6898ffc41889ea: function(arg0, arg1) {
            arg0.disable(arg1 >>> 0);
        },
        __wbg_disable_896f703cc44cf1e8: function(arg0, arg1) {
            arg0.disable(arg1 >>> 0);
        },
        __wbg_disconnect_99bdd53252c1a923: function(arg0) {
            arg0.disconnect();
        },
        __wbg_document_a28a21ae315de4ea: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_547d467e97529006: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_drawElements_20b18c4bf64fa976: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        },
        __wbg_drawElements_f0da90b9c827e09d: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        },
        __wbg_elementFromPoint_6c20f1acad715888: function(arg0, arg1, arg2) {
            const ret = arg0.elementFromPoint(arg1, arg2);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_elementFromPoint_d497b2a177e5ac7c: function(arg0, arg1, arg2) {
            const ret = arg0.elementFromPoint(arg1, arg2);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_enableVertexAttribArray_acf4abf519ab0114: function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enableVertexAttribArray_b4abeab358174fdb: function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enable_9328f475236428ef: function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        },
        __wbg_enable_f1131cfcbbb57556: function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        },
        __wbg_error_38bec0a78dd8ded8: function(arg0) {
            console.error(arg0);
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_error_bbcc95426a3167ad: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_fetch_1a731e18c5e21884: function(arg0, arg1) {
            const ret = arg0.fetch(arg1);
            return ret;
        },
        __wbg_fetch_9dad4fe911207b37: function(arg0) {
            const ret = fetch(arg0);
            return ret;
        },
        __wbg_files_49421c0860b80e49: function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_files_becc181f99e4c14c: function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_focus_93aead258d471c93: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_force_12fb0f147a1b59af: function(arg0) {
            const ret = arg0.force;
            return ret;
        },
        __wbg_generateMipmap_33b06e2e8457c7aa: function(arg0, arg1) {
            arg0.generateMipmap(arg1 >>> 0);
        },
        __wbg_generateMipmap_ae4fd1e23206c39f: function(arg0, arg1) {
            arg0.generateMipmap(arg1 >>> 0);
        },
        __wbg_getAttribLocation_085226b5f2506399: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getAttribLocation(arg1, getStringFromWasm0(arg2, arg3));
            return ret;
        },
        __wbg_getAttribLocation_bd2607e1da7ebe47: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getAttribLocation(arg1, getStringFromWasm0(arg2, arg3));
            return ret;
        },
        __wbg_getBoundingClientRect_dd4198d549ad0fd6: function(arg0) {
            const ret = arg0.getBoundingClientRect();
            return ret;
        },
        __wbg_getComputedStyle_032eef1be41bbff9: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getComputedStyle(arg1);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_8f1ff363618c55da: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getData_546274e368ce61dd: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getData(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getElementById_1a2b69d69d3a074f: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getError_18c9114d9a336b5b: function(arg0) {
            const ret = arg0.getError();
            return ret;
        },
        __wbg_getError_640aae86a11cebc8: function(arg0) {
            const ret = arg0.getError();
            return ret;
        },
        __wbg_getExtension_0a83dcb9303b3e8a: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getExtension_ce16f3780572b35e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getItem_203869d4a1ba1433: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getParameter_037149e897c929ad: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_getParameter_09ce4298daa62d31: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_getProgramInfoLog_b2d112da8cb8c5c5: function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramInfoLog_b4bc560fd6ea687d: function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramParameter_2b7693f9edfde93d: function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getProgramParameter_6dc3590345750abb: function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getPropertyValue_12e464ea4b1c3fe4: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getPropertyValue(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getRootNode_cbe0078ee615c20a: function(arg0) {
            const ret = arg0.getRootNode();
            return ret;
        },
        __wbg_getShaderInfoLog_57aaac3110ec22f3: function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderInfoLog_737b1be2c43195d8: function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderParameter_2b6f35d96d51cc82: function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getShaderParameter_cc12071135e57d45: function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getSupportedExtensions_3a75a2c3e17864bf: function(arg0) {
            const ret = arg0.getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getSupportedExtensions_92b6dc82a889082d: function(arg0) {
            const ret = arg0.getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getTime_f6ac312467f7cf09: function(arg0) {
            const ret = arg0.getTime();
            return ret;
        },
        __wbg_getUniformLocation_2e7496f74219fc19: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getUniformLocation_8d93a5f3de4232bf: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_22172cd3e3bb6c58: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_761e9cf5d605bcbd: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_cb1662031c6b2d85: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_ed0642c4b9d31ddf: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_f96702c6245e4ef9: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_unchecked_7d7babe32e9e6a54: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_has_3ec5c22db2e5237a: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_hash_da89606b87400cf8: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_headers_e08dcb5aa09b9a63: function(arg0) {
            const ret = arg0.headers;
            return ret;
        },
        __wbg_height_734034c3ff2654af: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_height_851bcd638e890abc: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_hidden_9d47d82daa2677c0: function(arg0) {
            const ret = arg0.hidden;
            return ret;
        },
        __wbg_host_142a4b1e170efc7f: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.host;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_hostname_a42f31a9081ac639: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hostname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_href_e111373f8d7ae763: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_id_3b6f51dc39f749d2: function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_identifier_db6b606c00c0fc57: function(arg0) {
            const ret = arg0.identifier;
            return ret;
        },
        __wbg_info_4e3339024d0fb613: function(arg0) {
            console.info(arg0);
        },
        __wbg_inlineSize_4d595b2867bf7d83: function(arg0) {
            const ret = arg0.inlineSize;
            return ret;
        },
        __wbg_instanceof_Document_fa497c324471af6b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Document;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Element_c4784e8a9522e087: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlAnchorElement_d244ac11562c8f69: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLAnchorElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlButtonElement_94e31ca453766121: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLButtonElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_3cec11b30b0d54e4: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlElement_43b8bbfdb4aaefd5: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlInputElement_ed700e6a857d360a: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLInputElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ResizeObserverEntry_2934c3da74688b5b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ResizeObserverEntry;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ResizeObserverSize_5b368efa5128c601: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ResizeObserverSize;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Response_06795eab66cc4036: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ShadowRoot_9508b4512523f14c: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ShadowRoot;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WebGl2RenderingContext_6502f76e53996a5e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WebGlRenderingContext_46d14e93c420ba70: function(arg0) {
            let result;
            try {
                result = arg0 instanceof WebGLRenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_c0fee4c064502536: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isActive_453c915c78ca9cf6: function(arg0) {
            const ret = arg0.isActive;
            return ret;
        },
        __wbg_isComposing_926558e85c1322f5: function(arg0) {
            const ret = arg0.isComposing;
            return ret;
        },
        __wbg_isComposing_fc3ec12836b74836: function(arg0) {
            const ret = arg0.isComposing;
            return ret;
        },
        __wbg_isSecureContext_2ad7065a08159f29: function(arg0) {
            const ret = arg0.isSecureContext;
            return ret;
        },
        __wbg_is_3ce118e1fc3aa47e: function(arg0, arg1) {
            const ret = Object.is(arg0, arg1);
            return ret;
        },
        __wbg_item_f76abbdc0cf17647: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_items_38e88bb85ee8204a: function(arg0) {
            const ret = arg0.items;
            return ret;
        },
        __wbg_iterator_de403ef31815a3e6: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_keyCode_7772df4809d7d3d8: function(arg0) {
            const ret = arg0.keyCode;
            return ret;
        },
        __wbg_key_7cfa20193d517a74: function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_lastModified_f3daedc0b92f6ee2: function(arg0) {
            const ret = arg0.lastModified;
            return ret;
        },
        __wbg_left_542bbb36968c6d4e: function(arg0) {
            const ret = arg0.left;
            return ret;
        },
        __wbg_length_06fc61606800dfdc: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_0c32cb8543c8e4c8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_2463cf44caac7a20: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_6e821edde497a532: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_c68a80a6327e9194: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_linkProgram_4a3a45fa4d8d09f0: function(arg0, arg1) {
            arg0.linkProgram(arg1);
        },
        __wbg_linkProgram_d86c69f8f86f3031: function(arg0, arg1) {
            arg0.linkProgram(arg1);
        },
        __wbg_localStorage_b1a71e6b7afdce21: function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_location_91b3fdbca3c76d9e: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_log_4c0baeb8af2f8f89: function(arg0) {
            console.log(arg0);
        },
        __wbg_matchMedia_1d8b96312cffb576: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_matches_86946499f934a7fd: function(arg0) {
            const ret = arg0.matches;
            return ret;
        },
        __wbg_metaKey_29a14f6b2fe9783e: function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        },
        __wbg_metaKey_cabf24bec9d42077: function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        },
        __wbg_name_c20e84db554b626b: function(arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_navigator_9b09ea705d03d227: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_0_bfa2ef4bc447daa2: function() {
            const ret = new Date();
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_3a112826a89cb962: function() { return handleError(function () {
            const ret = new Headers();
            return ret;
        }, arguments); },
        __wbg_new_40792555590ec35c: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_4f9fafbb3909af72: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_65733360f6737f38: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_6f89ec4f8aab68cc: function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(arg0);
            return ret;
        }, arguments); },
        __wbg_new_9abbf7148481485e: function() { return handleError(function () {
            const ret = new AbortController();
            return ret;
        }, arguments); },
        __wbg_new_a560378ea1240b14: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_a5a447223ac6ad64: function() { return handleError(function () {
            const ret = new FileReader();
            return ret;
        }, arguments); },
        __wbg_new_f3c9df4f38f3f798: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_from_slice_2580ff33d0d10520: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_with_record_from_str_to_blob_promise_50c718d98edeba27: function() { return handleError(function (arg0) {
            const ret = new ClipboardItem(arg0);
            return ret;
        }, arguments); },
        __wbg_new_with_str_and_init_f663b6d334baa878: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_new_with_u8_array_sequence_and_options_0ea871c78d13a6d8: function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_next_01132ed6134b8ef5: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_b3713ec761a9dbfd: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_now_2c44418ca0623664: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_now_e7c6795a7f81e10f: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_observe_aa0d17c78115be71: function(arg0, arg1, arg2) {
            arg0.observe(arg1, arg2);
        },
        __wbg_of_cc32e7afcce5ea8e: function(arg0) {
            const ret = Array.of(arg0);
            return ret;
        },
        __wbg_offsetTop_03ccc0dfd690579b: function(arg0) {
            const ret = arg0.offsetTop;
            return ret;
        },
        __wbg_open_a3e83bb9ce5c4f9d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_origin_068a8b12f26e8142: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.origin;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_performance_3fcf6e32a7e1ed0a: function(arg0) {
            const ret = arg0.performance;
            return ret;
        },
        __wbg_performance_5ed3f6a3bbe36d0d: function(arg0) {
            const ret = arg0.performance;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_pixelStorei_3dd51cd2a28442f6: function(arg0, arg1, arg2) {
            arg0.pixelStorei(arg1 >>> 0, arg2);
        },
        __wbg_pixelStorei_a5f8fc3966b8599d: function(arg0, arg1, arg2) {
            arg0.pixelStorei(arg1 >>> 0, arg2);
        },
        __wbg_port_97246809d787c307: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.port;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_preventDefault_9c72c03ba5e7d9c7: function(arg0) {
            arg0.preventDefault();
        },
        __wbg_protocol_45fde0ce406160d8: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.protocol;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_prototypesetcall_3e05eb9545565046: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_6bdbc990be5ac37b: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_abaf92f0bd4e80a4: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_df5a6dac26d818f3: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_readAsArrayBuffer_981694de4b45e06b: function() { return handleError(function (arg0, arg1) {
            arg0.readAsArrayBuffer(arg1);
        }, arguments); },
        __wbg_readPixels_6effecfcb3dc1144: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments); },
        __wbg_readPixels_9b75a1927b6caa46: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments); },
        __wbg_readPixels_e434d71b868f30c5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments); },
        __wbg_removeChild_874b23fe5a1ca1bb: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_e5033ab3bcad443c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_remove_b0ab1a7ea21cda2e: function(arg0) {
            arg0.remove();
        },
        __wbg_requestAnimationFrame_e1628778767f2bf2: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments); },
        __wbg_resolve_0a79de24e9d2267b: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_result_69b9c700f4d4f22f: function() { return handleError(function (arg0) {
            const ret = arg0.result;
            return ret;
        }, arguments); },
        __wbg_right_7bb2640e243f1b81: function(arg0) {
            const ret = arg0.right;
            return ret;
        },
        __wbg_scissor_2ab796946944a395: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.scissor(arg1, arg2, arg3, arg4);
        },
        __wbg_scissor_6a7028a46e34c58f: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.scissor(arg1, arg2, arg3, arg4);
        },
        __wbg_search_4e5c22f751154ad5: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_setAttribute_5799fb5befe29601: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setItem_67573afec8996fe4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setProperty_872b034b6bcc67cd: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setTimeout_ef24d2fc3ad97385: function() { return handleError(function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_setTimeout_f757f00851f76c42: function(arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        },
        __wbg_set_8ee2d34facb8466e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_accept_59891065e5b3f283: function(arg0, arg1, arg2) {
            arg0.accept = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_autofocus_76fe163909c5a3cf: function() { return handleError(function (arg0, arg1) {
            arg0.autofocus = arg1 !== 0;
        }, arguments); },
        __wbg_set_body_a304d09cb50cefbe: function(arg0, arg1) {
            arg0.body = arg1;
        },
        __wbg_set_box_b3facca2aa9c2ac2: function(arg0, arg1) {
            arg0.box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
        },
        __wbg_set_cache_cc687e2b96e9608c: function(arg0, arg1) {
            arg0.cache = __wbindgen_enum_RequestCache[arg1];
        },
        __wbg_set_className_7ff38385280f6997: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_credentials_7693e63055f5e838: function(arg0, arg1) {
            arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
        },
        __wbg_set_download_6fb6f1e8e782dd11: function(arg0, arg1, arg2) {
            arg0.download = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_headers_6ab1105e542834e2: function(arg0, arg1) {
            arg0.headers = arg1;
        },
        __wbg_set_height_26ab95ff99e2b620: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_href_72e78ccb3b2c5988: function(arg0, arg1, arg2) {
            arg0.href = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_id_b1cffd0a170935f4: function(arg0, arg1, arg2) {
            arg0.id = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerHTML_7e29b346becaeb8b: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerText_20f0b35c707c016a: function(arg0, arg1, arg2) {
            arg0.innerText = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_method_1971272fe557e972: function(arg0, arg1, arg2) {
            arg0.method = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_mode_d1b643087602281a: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        },
        __wbg_set_multiple_390ca167c25b3a68: function(arg0, arg1) {
            arg0.multiple = arg1 !== 0;
        },
        __wbg_set_once_21b4f52a7651545b: function(arg0, arg1) {
            arg0.once = arg1 !== 0;
        },
        __wbg_set_onclick_60f72f3e80d65322: function(arg0, arg1) {
            arg0.onclick = arg1;
        },
        __wbg_set_onload_130e9637450f8179: function(arg0, arg1) {
            arg0.onload = arg1;
        },
        __wbg_set_signal_8564a226c5c6853c: function(arg0, arg1) {
            arg0.signal = arg1;
        },
        __wbg_set_tabIndex_21aab214ff7b4d6e: function(arg0, arg1) {
            arg0.tabIndex = arg1;
        },
        __wbg_set_type_1c9054396ddd2edb: function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_type_ef754f25329c9096: function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_1e50ba479347b273: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_width_81fa781e87b17891: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_shaderSource_c235f38ba5b536d3: function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        },
        __wbg_shaderSource_cae157a332281ae7: function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        },
        __wbg_shiftKey_44bc0e4535e829c0: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_shiftKey_4f414ec7c42beae6: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_signal_9172c3282bfba2f5: function(arg0) {
            const ret = arg0.signal;
            return ret;
        },
        __wbg_size_7306c9406e13bf29: function(arg0) {
            const ret = arg0.size;
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_stack_84752149fa5763ad: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_a1248013d790bf5f: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_f2e0f995a21329ff: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_24f78b6d23f286ea: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_59fd959c540fe405: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_status_44ecb0ac1da253f4: function(arg0) {
            const ret = arg0.status;
            return ret;
        },
        __wbg_stopPropagation_000e2055cf404c4c: function(arg0) {
            arg0.stopPropagation();
        },
        __wbg_stringify_a2c39d991e1bf91d: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_style_fbb0b56f71e97cf5: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_texImage2D_29ce63ed3c9e7fd2: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texImage2D_35dad0302576d81d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texImage2D_b708a52e67380671: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texParameteri_2ae301ef0bcf17eb: function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        },
        __wbg_texParameteri_51f89620521fe4f5: function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        },
        __wbg_texSubImage2D_403156f007363972: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texSubImage2D_9489e066941c87f5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texSubImage2D_efd0d5d4f44425c3: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_then_00eed3ac0b8e82cb: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_a0c8db0381c8994c: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_top_a1a3c9fce4b7bf79: function(arg0) {
            const ret = arg0.top;
            return ret;
        },
        __wbg_touches_077e4a58ee8e1500: function(arg0) {
            const ret = arg0.touches;
            return ret;
        },
        __wbg_type_519bcbe4568b18ef: function(arg0, arg1) {
            const ret = arg1.type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_type_8f15c453c9572b22: function(arg0, arg1) {
            const ret = arg1.type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_uniform1i_a2d71c729752832f: function(arg0, arg1, arg2) {
            arg0.uniform1i(arg1, arg2);
        },
        __wbg_uniform1i_acce06d190ce18d5: function(arg0, arg1, arg2) {
            arg0.uniform1i(arg1, arg2);
        },
        __wbg_uniform2f_186549d813184ee8: function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        },
        __wbg_uniform2f_d2e5d535614031f0: function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        },
        __wbg_url_95d8a83d33709572: function(arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_useProgram_a2f83fab51c79f54: function(arg0, arg1) {
            arg0.useProgram(arg1);
        },
        __wbg_useProgram_f79c775d2e8824a9: function(arg0, arg1) {
            arg0.useProgram(arg1);
        },
        __wbg_userActivation_16761a917b187328: function(arg0) {
            const ret = arg0.userActivation;
            return ret;
        },
        __wbg_userAgent_d58193cc32293b16: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.userAgent;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_value_7f6052747ccf940f: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_value_c88240b8ee29611a: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_vertexAttribPointer_63d8611810159fd4: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_vertexAttribPointer_7db76295987fda72: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_viewport_1ac0b434f13a485b: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_viewport_de5bbf3f5c97bfcf: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_warn_2b0a27f629a4bb1e: function(arg0) {
            console.warn(arg0);
        },
        __wbg_width_4e12e0c19210bcc8: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_width_80cea93fc7f63070: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_writeText_622762687282c553: function(arg0, arg1, arg2) {
            const ret = arg0.writeText(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_write_00f6c35f2c69124a: function(arg0, arg1) {
            const ret = arg0.write(arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 2087, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___wasm_bindgen_c94c3efd945c55f7___JsValue__core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsError___true_);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Array<any>")], shim_idx: 1073, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 1073, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true__2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 1076, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsValue___true_);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 597, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 610, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__1_);
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 810, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__2_);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
            const ret = getArrayF32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I16)) -> NamedExternref("Int16Array")`.
            const ret = getArrayI16FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000a: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I32)) -> NamedExternref("Int32Array")`.
            const ret = getArrayI32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000b: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I8)) -> NamedExternref("Int8Array")`.
            const ret = getArrayI8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000c: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U16)) -> NamedExternref("Uint16Array")`.
            const ret = getArrayU16FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000d: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
            const ret = getArrayU32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000e: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_000000000000000f: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./zcb3_bg.js": import0,
    };
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__1_(arg0, arg1) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__1_(arg0, arg1);
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__2_(arg0, arg1) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke_______true__2_(arg0, arg1);
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsValue___true_(arg0, arg1) {
    const ret = wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsValue___true_(arg0, arg1);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true_(arg0, arg1, arg2);
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true__2(arg0, arg1, arg2) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Array______true__2(arg0, arg1, arg2);
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___wasm_bindgen_c94c3efd945c55f7___JsValue__core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___wasm_bindgen_c94c3efd945c55f7___JsValue__core_d994a3bb74c02bb6___result__Result_____wasm_bindgen_c94c3efd945c55f7___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_c94c3efd945c55f7___convert__closures_____invoke___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined___js_sys_9efd84f9a39d16f0___Function_fn_wasm_bindgen_c94c3efd945c55f7___JsValue_____wasm_bindgen_c94c3efd945c55f7___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_RequestCache = ["default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"];


const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];


const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];


const __wbindgen_enum_ResizeObserverBoxOptions = ["border-box", "content-box", "device-pixel-content-box"];

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getArrayU16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedInt16ArrayMemory0 = null;
function getInt16ArrayMemory0() {
    if (cachedInt16ArrayMemory0 === null || cachedInt16ArrayMemory0.byteLength === 0) {
        cachedInt16ArrayMemory0 = new Int16Array(wasm.memory.buffer);
    }
    return cachedInt16ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

let cachedInt8ArrayMemory0 = null;
function getInt8ArrayMemory0() {
    if (cachedInt8ArrayMemory0 === null || cachedInt8ArrayMemory0.byteLength === 0) {
        cachedInt8ArrayMemory0 = new Int8Array(wasm.memory.buffer);
    }
    return cachedInt8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint16ArrayMemory0 = null;
function getUint16ArrayMemory0() {
    if (cachedUint16ArrayMemory0 === null || cachedUint16ArrayMemory0.byteLength === 0) {
        cachedUint16ArrayMemory0 = new Uint16Array(wasm.memory.buffer);
    }
    return cachedUint16ArrayMemory0;
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedInt16ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedInt8ArrayMemory0 = null;
    cachedUint16ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('zcb3_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
