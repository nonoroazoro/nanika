#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>

// Owned native test window only. Never activate the app or inject global input.
@interface WKEngineProbe : NSObject <WKURLSchemeHandler, WKScriptMessageHandler, WKNavigationDelegate>
@property(nonatomic, copy) NSString *root;
@property(nonatomic, copy) NSString *mode;
@property(nonatomic, strong) WKWebView *webview;
@property(nonatomic, strong) NSPanel *panel;
@end

@implementation WKEngineProbe
- (void)webView:(WKWebView *)view startURLSchemeTask:(id<WKURLSchemeTask>)task {
    NSString *file = [[self.root stringByAppendingPathComponent:task.request.URL.path] stringByStandardizingPath];
    if (![file hasPrefix:[self.root stringByAppendingString:@"/"]]) {
        [task didFailWithError:[NSError errorWithDomain:@"probe" code:403 userInfo:nil]];
        return;
    }
    NSData *data = [NSData dataWithContentsOfFile:file];
    if (!data) {
        [task didFailWithError:[NSError errorWithDomain:@"probe" code:404 userInfo:nil]];
        return;
    }
    NSDictionary *types = @{@"html": @"text/html", @"css": @"text/css", @"js": @"text/javascript", @"json": @"application/json", @"wasm": @"application/wasm"};
    NSHTTPURLResponse *response = [[NSHTTPURLResponse alloc] initWithURL:task.request.URL statusCode:200 HTTPVersion:@"HTTP/1.1" headerFields:@{@"Content-Type": types[file.pathExtension] ?: @"application/octet-stream", @"Access-Control-Allow-Origin": @"tauri://localhost"}];
    [task didReceiveResponse:response];
    [task didReceiveData:data];
    [task didFinish];
}
- (void)webView:(WKWebView *)view stopURLSchemeTask:(id<WKURLSchemeTask>)task {}
- (void)webView:(WKWebView *)view didFinishNavigation:(WKNavigation *)navigation {
    fprintf(stderr, "Native probe: page loaded (%s).\n", self.mode.UTF8String);
    // Keep the async evaluation alive for the complete run, including hidden-idle checks.
    NSString *script =
        @"const pause=ms=>new Promise(r=>setTimeout(r,ms));"
         "const frame=()=>new Promise(r=>requestAnimationFrame(r));"
         "const resized=async width=>{for(let i=0;i<120&&innerWidth!==width;i++)await frame();if(innerWidth!==width)throw Error('Native viewport size did not arrive');await frame();await frame()};"
         "const native=action=>new Promise(r=>{window.__syntaxNativeComplete=r;webkit.messageHandlers.native.postMessage(action)});"
         // Native event delivery and JavaScript evaluation use different WebKit queues.
         "const caret=async(anchor,focus)=>{let state;for(let i=0;i<120;i++){await frame();state=window.syntaxInteractionState();if(state.anchor===anchor&&state.focus===focus)return state}return state};"
         "const entry=worker?'runSyntaxWorker':latency?'runSyntaxLatency':'runSyntaxBenchmark';"
         "for(let i=0;i<500&&!window[entry];i++)await pause(10);"
         "webkit.messageHandlers.log.postMessage('Native probe: benchmark started, visibility='+document.visibilityState);"
         "const result=await window[entry]();"
         "if(worker||latency)return JSON.stringify(result);"
         "webkit.messageHandlers.log.postMessage('Native probe: component complete; keyboard checks');"
         "const prepared=await window.prepareSyntaxInteraction();"
         "await native('right');"
         "const moved=await caret(prepared.start+6,prepared.start+6);"
         "await native('left');"
         "const selected=await caret(prepared.start+6,prepared.start);"
         "await native('edit');await frame();await frame();"
         "const after=window.syntaxInteractionState();"
         "result.nativeInteraction={caret:moved.anchor===prepared.start+6&&moved.focus===prepared.start+6,"
         "backwards:selected.anchor===prepared.start+6&&selected.focus===prepared.start,"
         "readOnly:after.text===prepared.text,focused:after.focused};"
         "if(Object.values(result.nativeInteraction).some(v=>!v))throw Error('Native keyboard failed: '+JSON.stringify(result.nativeInteraction));"
         "const before=await window.prepareSyntaxNativeLifecycle();"
         "webkit.messageHandlers.log.postMessage('Native probe: keyboard complete; window lifecycle');"
         "if(before){"
         "await native('hide');"
         "for(let i=0;i<100&&document.visibilityState!=='hidden';i++)await pause(10);"
         "const hidden=window.syntaxNativeState();await pause(250);const idle=window.syntaxNativeState();"
         "await native('show');await frame();await frame();const restored=window.syntaxNativeState();"
         "result.nativeLifecycle={before,hidden,idle,restored};"
         "if(!before.visible||before.live!==1||hidden.visible||idle.live!==0||idle.created!==hidden.created||idle.received!==hidden.received||idle.reads!==hidden.reads||!restored.visible||!restored.colors||restored.created!==idle.created)throw Error('Native hide/restore failed: '+JSON.stringify(result.nativeLifecycle));"
         "const selection=getSelection();const source=document.querySelector('.text-document');"
         "selection.setBaseAndExtent(source,source.childNodes.length,source,0);"
         "const exact=selection.getRangeAt(0).toString();"
         "await native('resize');await resized(480);"
         "const small=window.verifySyntaxNativeViewport();"
         "await native('restore');await resized(640);"
         "const large=window.verifySyntaxNativeViewport();"
         "result.nativeResize={small,large,selection:selection.getRangeAt(0).toString()===exact};"
         "if(large.mismatches){await pause(100);result.nativeResize.delayed=window.verifySyntaxNativeViewport()}"
         "if(!result.nativeResize.selection||small.mismatches||large.mismatches||!small.checked||!large.checked||small.width>=large.width)throw Error('Native resize failed: '+JSON.stringify(result.nativeResize));"
         "selection.removeAllRanges();"
         "await native('light');await frame();await frame();const light=window.syntaxNativeAppearance();"
         "await native('dark');await frame();await frame();const dark=window.syntaxNativeAppearance();"
         "result.nativeAppearance={light,dark};"
         "if(light.dark||!dark.dark||light.color===dark.color||light.height!==dark.height||light.created!==dark.created)throw Error('Native appearance failed: '+JSON.stringify(result.nativeAppearance));"
         "}"
         "return JSON.stringify(result);";
    [view callAsyncJavaScript:script arguments:@{@"worker": @([self.mode isEqualToString:@"worker"]), @"latency": @([self.mode isEqualToString:@"latency"])} inFrame:nil inContentWorld:WKContentWorld.pageWorld completionHandler:^(id value, NSError *error) {
        if (error) { fprintf(stderr, "%s\n", error.description.UTF8String); exit(2); }
        NSMutableDictionary *result = [NSJSONSerialization JSONObjectWithData:[[value description] dataUsingEncoding:NSUTF8StringEncoding] options:NSJSONReadingMutableContainers error:nil];
        NSScreen *screen = self.panel.screen ?: NSScreen.mainScreen;
        result[@"nativeHost"] = @{
            @"activated": @(NSApp.active),
            @"windowed": @(self.panel != nil),
            @"screenMaximumFps": @(screen.maximumFramesPerSecond),
            @"screenMinimumRefreshInterval": @(screen.minimumRefreshInterval),
            @"screenMaximumRefreshInterval": @(screen.maximumRefreshInterval)
        };
        NSData *json = [NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
        printf("%s\n", [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
        fflush(stdout);
        [self.panel orderOut:nil];
        exit(0);
    }];
}
- (void)webView:(WKWebView *)view didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    fprintf(stderr, "%s\n", error.description.UTF8String); exit(2);
}
- (void)webView:(WKWebView *)view didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    fprintf(stderr, "%s\n", error.description.UTF8String); exit(2);
}
- (void)webViewWebContentProcessDidTerminate:(WKWebView *)view {
    fprintf(stderr, "Native WebContent process terminated.\n"); exit(2);
}
- (void)userContentController:(WKUserContentController *)controller didReceiveScriptMessage:(WKScriptMessage *)message {
    if ([message.name isEqualToString:@"log"]) {
        fprintf(stderr, "%s\n", [[message.body description] UTF8String]);
        return;
    }
    NSString *action = message.body;
    if ([action isEqualToString:@"hide"]) [self.panel orderOut:nil];
    else if ([action isEqualToString:@"show"]) [self.panel orderFrontRegardless];
    else if ([action isEqualToString:@"resize"]) [self.panel setContentSize:NSMakeSize(480, 480)];
    else if ([action isEqualToString:@"restore"]) [self.panel setContentSize:NSMakeSize(640, 600)];
    else if ([action isEqualToString:@"light"]) self.panel.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
    else if ([action isEqualToString:@"dark"]) self.panel.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
    else if ([action isEqualToString:@"right"] || [action isEqualToString:@"left"]) {
        BOOL left = [action isEqualToString:@"left"];
        NSString *characters = [NSString stringWithFormat:@"%C", (unichar)(left ? NSLeftArrowFunctionKey : NSRightArrowFunctionKey)];
        for (NSInteger index = 0; index < 6; index++) {
            [self _key:characters code:left ? 123 : 124 modifiers:NSEventModifierFlagFunction | NSEventModifierFlagNumericPad | (left ? NSEventModifierFlagShift : 0)];
        }
    } else if ([action isEqualToString:@"edit"]) {
        [self _key:@"\177" code:51 modifiers:0];
        [self _key:@"x" code:7 modifiers:0];
    } else { fprintf(stderr, "Unknown native probe command.\n"); exit(2); }
    [self.webview evaluateJavaScript:@"window.__syntaxNativeComplete()" completionHandler:nil];
}
- (void)_key:(NSString *)characters code:(unsigned short)code modifiers:(NSEventModifierFlags)modifiers {
    // Deliver only to this window's responder, never NSApp or the system event stream.
    [self.panel makeFirstResponder:self.webview];
    for (NSNumber *kind in @[@(NSEventTypeKeyDown), @(NSEventTypeKeyUp)]) {
        NSEvent *event = [NSEvent keyEventWithType:kind.unsignedIntegerValue location:NSZeroPoint modifierFlags:modifiers timestamp:NSProcessInfo.processInfo.systemUptime windowNumber:self.panel.windowNumber context:nil characters:characters charactersIgnoringModifiers:characters isARepeat:NO keyCode:code];
        if (kind.unsignedIntegerValue == NSEventTypeKeyDown) [self.panel.firstResponder keyDown:event];
        else [self.panel.firstResponder keyUp:event];
    }
}
@end

int main(int argc, const char **argv) {
    @autoreleasepool {
        if (argc != 3) { fprintf(stderr, "Usage: WKEngineProbe <build-directory> worker|original|off|on|latency\n"); return 2; }
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyProhibited];
        WKEngineProbe *probe = [WKEngineProbe new];
        probe.root = [[[NSString stringWithUTF8String:argv[1]] stringByResolvingSymlinksInPath] stringByStandardizingPath];
        probe.mode = [NSString stringWithUTF8String:argv[2]];
        if (![@[@"worker", @"original", @"off", @"on", @"latency"] containsObject:probe.mode]) return 2;
        WKWebViewConfiguration *configuration = [WKWebViewConfiguration new];
        [configuration setURLSchemeHandler:probe forURLScheme:@"tauri"];
        [configuration.userContentController addScriptMessageHandler:probe name:@"log"];
        [configuration.userContentController addScriptMessageHandler:probe name:@"native"];
        [configuration.userContentController addUserScript:[[WKUserScript alloc] initWithSource:@"console.error=(...args)=>webkit.messageHandlers.log.postMessage(args.map(String).join(' '));addEventListener('syntax-benchmark-progress',e=>webkit.messageHandlers.log.postMessage(e.detail));addEventListener('error',e=>webkit.messageHandlers.log.postMessage(e.message||('Resource load failed: '+e.target?.src)),true);addEventListener('securitypolicyviolation',e=>webkit.messageHandlers.log.postMessage(e.violatedDirective+': '+e.blockedURI));" injectionTime:WKUserScriptInjectionTimeAtDocumentStart forMainFrameOnly:YES]];
        probe.webview = [[WKWebView alloc] initWithFrame:NSMakeRect(0, 0, 640, 600) configuration:configuration];
        if (![probe.mode isEqualToString:@"worker"]) {
            NSRect screen = NSScreen.mainScreen.visibleFrame;
            probe.panel = [[NSPanel alloc] initWithContentRect:NSMakeRect(NSMaxX(screen) - 664, NSMinY(screen) + 24, 640, 600) styleMask:NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel backing:NSBackingStoreBuffered defer:NO];
            probe.panel.ignoresMouseEvents = YES;
            probe.panel.hidesOnDeactivate = NO;
            // Keep the test surface unoccluded without making it key or activating the app.
            // WebKit suspends animation frames for an occluded native window.
            probe.panel.level = NSFloatingWindowLevel;
            probe.panel.collectionBehavior = NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary | NSWindowCollectionBehaviorIgnoresCycle;
            probe.panel.contentView = probe.webview;
            [probe.panel orderFrontRegardless];
        }
        probe.webview.navigationDelegate = probe;
        NSString *url = [NSString stringWithFormat:@"tauri://localhost/index.html?syntax=%@&implementation=%@", [@[@"on", @"latency"] containsObject:probe.mode] ? @"on" : @"off", probe.mode];
        [probe.webview loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:url]]];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 90 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{ fprintf(stderr, "Native probe timed out.\n"); exit(2); });
        [NSApp run];
    }
    return 0;
}
