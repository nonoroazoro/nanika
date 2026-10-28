import { join } from "node:path";

import { verifyScrollGutters } from "../scroll-area.ts";

import type { BrowserCommand } from "../BrowserCommand.ts";

/**
 * Exercises the real launcher surface with deterministic bridge fixtures.
 *
 * @param browser Owned headless session
 * @param output Screenshot directory
 */
export async function verifyLauncher(browser: BrowserCommand, output: string): Promise<void>
{
    await _verifyScrollLayout(browser, output);
    await _verifyLongLabels(browser, output);
    const search = "input[aria-label='Search for apps and commands']";
    await browser("fill", search, "Empty");
    await browser("wait", "[role=option][aria-selected=true]");
    await browser("eval", "window.setCatalogWarning(true)");
    await browser("wait", "--text", "The application catalog could not be refreshed.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector("input[aria-label='Search for apps and commands']");
        input.setSelectionRange(1, 4);
        window.retainedRootInput = input;
        window.retainedRootRow = document.querySelector('[role=option][aria-selected=true]');
        window.retainedQueryCount = document.documentElement.dataset.queryCount;
        window.retainedResourceReads = document.documentElement.dataset.localizationReads;
        window.retainedWarning = document.querySelector('.warning[role=status]');
        window.retainedWarning.scrollTop = 80;
        window.retainedWarningDetail = window.retainedWarning.textContent.split('EACCES:')[1];
        window.retainedRootScroll = document.querySelector('.root-viewport').scrollTop;
        if (window.retainedRootRow.querySelector('mark')?.textContent !== 'Empty') throw new Error('English title match was not highlighted');
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "--text", "清倒废纸篓");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector("input[aria-label='搜索应用和命令']");
        const row = document.querySelector('[role=option][aria-selected=true]');
        if (input?.placeholder !== '搜索应用和命令' || !document.querySelector('main[aria-label="Nanika 启动器"]')) throw new Error('Launcher accessibility labels did not translate');
        if (input !== window.retainedRootInput || document.activeElement !== input || input.value !== 'Empty' || input.selectionStart !== 1 || input.selectionEnd !== 4) throw new Error('Language change reset root input state');
        if (row !== window.retainedRootRow || row.querySelector('.collection-title').textContent !== '清倒废纸篓' || row.querySelector('.subtitle').textContent !== '系统') throw new Error('Root candidate translation lost row identity');
        if (row.querySelector('mark')) throw new Error('English alias offsets were applied to Chinese title');
        if (document.documentElement.dataset.queryCount !== window.retainedQueryCount || document.documentElement.dataset.localizationReads !== window.retainedResourceReads) throw new Error('Warm root language change queried or loaded resources');
        const warning = document.querySelector('.warning[role=status]');
        if (warning !== window.retainedWarning || !warning.textContent.includes('无法刷新应用目录，请查看详情后重试。') || !warning.textContent.includes('某项系统服务不可用')) throw new Error('Retained catalog warning did not translate');
        if (warning.textContent.split('EACCES:')[1] !== window.retainedWarningDetail) throw new Error('Translation changed original diagnostic');
        if (warning.scrollTop !== 80) throw new Error('Translation changed diagnostic scroll to ' + warning.scrollTop);
        if (warning.getBoundingClientRect().height > innerHeight * .25 + 1 || warning.scrollHeight <= warning.clientHeight || row.getBoundingClientRect().bottom > innerHeight) throw new Error('Catalog diagnostic displaced usable results');
        if (document.querySelector('.root-viewport').scrollTop !== window.retainedRootScroll) throw new Error('Warning translation reset scroll');
        warning.scrollTop = 0;
        return true;
    })()`
    );
    await browser("screenshot", join(output, "root-candidate-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "--text", "Empty Trash");
    await browser("wait", "--text", "The application catalog could not be refreshed.");
    await browser("eval", "window.setCatalogWarning(false)");
    await browser(
        "eval",
        `(async () => {
        const search = document.querySelector("input[aria-label='Search for apps and commands']");
        search.dispatchEvent(new KeyboardEvent('keydown', {
            key: 'Enter', code: 'Enter', keyCode: 229, isComposing: false, bubbles: true, cancelable: true
        }));
        await new Promise(requestAnimationFrame);
        if (document.querySelector('[role=alertdialog]')) throw new Error('IME Enter invoked a search result');
        return true;
    })()`
    );
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser("screenshot", join(output, "system-confirmation.png"));
    await browser(
        "eval",
        `(() => {
        window.retainedConfirmation = document.querySelector('[role=alertdialog]');
        window.retainedConfirmationFocus = document.activeElement;
        window.retainedResourceReads = document.documentElement.dataset.localizationReads;
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "--text", "清倒废纸篓？");
    await browser(
        "eval",
        `(() => {
        const dialog = document.querySelector('[role=alertdialog]');
        if (dialog !== window.retainedConfirmation || document.activeElement !== window.retainedConfirmationFocus) throw new Error('Root confirmation lost identity or focus');
        if (![...dialog.querySelectorAll('button')].some(button => button.textContent.trim() === '取消')) throw new Error('Confirmation cancel did not translate');
        if (!dialog.textContent.includes('废纸篓中的所有项目将被永久删除。此操作无法撤销。')) throw new Error('Root confirmation consequences did not translate');
        if (![...dialog.querySelectorAll('button')].some(button => button.textContent.trim() === '清倒废纸篓')) throw new Error('Root confirmation action title did not translate');
        if (document.documentElement.dataset.localizationReads !== window.retainedResourceReads) throw new Error('Root confirmation language change fetched resources');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "system-confirmation-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "--text", "Empty Trash?");
    await browser("press", "Escape");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    for (const reduced of [false, true])
    {
        await browser("set", "media", "dark", ...(reduced ? ["reduced-motion"] : []));
        await _verifyConfirmationExit(browser, true, reduced);
        await _verifyConfirmationExit(browser, false, reduced);
    }
    await browser("set", "media", "dark");
    await _verifyConfirmationInterruption(browser);
    await browser("fill", search, "Clipboard");
    await browser(
        "eval",
        `(() => {
        window.retainedRootInput = document.querySelector("input[aria-label='Search for apps and commands']");
        window.failNextLocalizationRead();
        return true;
    })()`
    );
    await browser("press", "Enter");
    await browser("wait", "--text", "The translations belong to a different package.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector("input[aria-label='Search for apps and commands']");
        if (input !== window.retainedRootInput || input.value !== 'Clipboard') throw new Error('A failed resource read discarded the last coherent UI');
        input.focus(); input.setSelectionRange(1, 4);
        window.retainedResourceFailure = document.querySelector('.operation-failure');
        window.failedResourceReadCount = document.documentElement.dataset.localizationReads;
        window.failedResourceQueryCount = document.documentElement.dataset.queryCount;
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "--text", "翻译属于其他扩展包");
    await browser(
        "eval",
        `(() => {
        const error = document.querySelector('.operation-failure');
        const input = window.retainedRootInput;
        if (error !== window.retainedResourceFailure || !error.textContent.includes('Original resource <diagnostic>')) throw new Error('Language replaced the resource failure');
        if (document.activeElement !== input || input.value !== 'Clipboard' || input.selectionStart !== 1 || input.selectionEnd !== 4) throw new Error('Resource error translation changed input');
        if (document.documentElement.dataset.localizationReads !== window.failedResourceReadCount || document.documentElement.dataset.queryCount !== window.failedResourceQueryCount) throw new Error('Language retried a resource failure');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-resource-error-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "--text", "The translations belong to a different package.");
    await browser("find", "role", "button", "click", "--name", "Try again", "--exact");
    await browser("wait", "--text", "Code preview");
    await browser("snapshot", "-i");
    await browser("screenshot", join(output, "extension.png"));
    await browser("find", "role", "button", "click", "--name", "\u2068Clipboard History\u2069 menu", "--exact");
    await browser("wait", "[role=menu]");
    await verifyScrollGutters(browser);
    await browser("screenshot", join(output, "extension-menu.png"));
    await browser(
        "eval",
        `(() => {
        window.retainedResourceReads = document.documentElement.dataset.localizationReads;
        window.retainedExtensionMenu = document.querySelector('[role=menu]');
        window.retainedMenuFocus = document.activeElement;
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "button[aria-label='\u2068剪贴板历史\u2069菜单']");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.localizationReads !== window.retainedResourceReads) throw new Error('Warm locale change fetched resources');
        if (document.querySelector('[role=menu]') !== window.retainedExtensionMenu) throw new Error('Locale replaced the open menu');
        if (document.activeElement !== window.retainedMenuFocus) throw new Error('Locale moved menu focus');
        if (![...document.querySelectorAll('[role=menuitem]')].some(item => item.textContent.trim().startsWith('设置'))) throw new Error('Host Settings action did not translate inside the package menu');
        if (!document.querySelector('[role=menu]').textContent.includes('外观') || !document.querySelector('[role=menuitemcheckbox]').textContent.includes('语法高亮')) throw new Error('Configuration menu text did not translate');
        if (!document.querySelector('[role=menu]').textContent.includes('清除历史记录')) throw new Error('The shared action label did not translate');
        if (!document.querySelector('.extension-menu-identity').textContent.includes('剪贴板历史')) throw new Error('Package menu identity did not translate');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "extension-menu-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "button[aria-label='\u2068Clipboard History\u2069 menu']");
    await browser("find", "role", "menuitemcheckbox", "click", "--name", "Syntax highlighting", "--exact");
    await browser("wait", "--fn", "!document.querySelector('[role=menu]')");
    await browser("find", "role", "button", "click", "--name", "\u2068Clipboard History\u2069 menu", "--exact");
    await browser("wait", "[role=menuitemcheckbox][aria-checked=false]");
    await _verifyConfigurationFailure(browser, output);
    await browser("find", "role", "menuitem", "click", "--name", "Settings", "--exact");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser("screenshot", join(output, "extension-settings.png"));
    await _verifyLocalizedList(browser, output);
    await _verifyLocalizedFiles(browser, output);
    await _verifyTextReadFailure(browser, output);
    await _verifyInvocationFailure(browser, output);
    await _verifyInvocationFailure(browser, output, true);
    await _verifyViewFailure(browser, output);
    await _verifyStartupFailure(browser, output);
    await _verifyStartupDiagnosticFailure(browser);
    await _verifyIsolatedTitle(browser, output);
    await _verifyQueryFailure(browser, output);
    await _verifyActionShortcuts(browser, output);
}

async function _verifyViewFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=view-error'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", "input[placeholder='Search clipboard history']");
    await browser("fill", "input[placeholder='Search clipboard history']", "retained query");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('.extension-view .content')?.getAttribute('aria-busy') === 'false'"
    );
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('.extension-view input');
        input.focus(); input.setSelectionRange(2, 6);
        const pane = document.querySelector('.extension-viewport'); pane.scrollTop = 300;
        window.retainedFailedView = { input, pane, row: document.querySelector('[role=option][aria-selected=true]'),
            source: document.querySelector('.text-preview'), reads: document.documentElement.dataset.localizationReads };
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "Text", "--exact");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('.extension-view .content')?.getAttribute('aria-busy') === 'true'"
    );
    await browser(
        "eval",
        "window.retainedViewEvents = document.documentElement.dataset.viewEventCount; window.changeLauncherLanguage('zh-Hans'); true"
    );
    await browser("wait", "input[placeholder='搜索剪贴板历史']");
    await browser("eval", "window.completeViewFailure(); true");
    await browser("wait", "--text", "无法更新剪贴板历史");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedFailedView;
        if (document.querySelector('.text-preview') !== kept.source || document.querySelector('[role=option][aria-selected=true]') !== kept.row) throw new Error('Rejected view replaced resident content');
        if (document.activeElement !== kept.input || kept.input.value !== 'retained query' || kept.input.selectionStart !== 2 || kept.input.selectionEnd !== 6) throw new Error('Rejected view reset query editing');
        if (kept.pane.scrollTop !== 300 || !document.querySelector('.filter button[aria-pressed=true]').textContent.includes('所有类型')) throw new Error('Rejected view changed scroll or filter');
        if (!document.querySelector('.extension-view .error').textContent.includes('EACCES: /Example/历史.db')) throw new Error('View diagnostic lost');
        window.changeLauncherLanguage('en'); return true;
    })()`
    );
    await browser("wait", "--text", "Clipboard History could not be updated.");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.viewEventCount !== window.retainedViewEvents || document.documentElement.dataset.localizationReads !== window.retainedFailedView.reads) throw new Error('Error translation repeated a view request');
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "--text", "无法更新剪贴板历史");
    await browser("screenshot", join(output, "view-error-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "文本", "--exact");
    await browser(
        "wait",
        "--fn",
        "!document.querySelector('.extension-view .error') && document.querySelector('.filter button[aria-pressed=true]')?.textContent.trim() === '文本'"
    );
    await browser("click", ".extension-view button.back");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('.extension-view .content')?.getAttribute('aria-busy') === 'true'"
    );
    await browser("eval", "window.completeViewFailure(); true");
    await browser("wait", "--text", "释放新视图也失败了：");
    await browser(
        "eval",
        `(() => {
        const error = document.querySelector('.extension-view .error');
        if (!error.textContent.includes('打开的视图过多') || !error.textContent.includes('无法关闭剪贴板历史') || !error.textContent.includes('EACCES: /Example/历史.db')) throw new Error('Cleanup discarded one of its failures');
        if (document.querySelector('.text-preview') !== window.retainedFailedView.source) throw new Error('Failed close discarded current view');
        const view = document.querySelector('.extension-view');
        if (error.scrollHeight <= error.clientHeight || error.clientHeight > view.clientHeight * 0.4) throw new Error('Long diagnostic escaped its scrollable error area');
        const back = view.querySelector('button.back').getBoundingClientRect();
        if (back.top < 0 || back.bottom > innerHeight || view.querySelector('button.back').disabled) throw new Error('Error obscured recovery');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "view-cleanup-error-chinese.png"));
    await browser("click", ".extension-view button.back");
    await browser("wait", "input[aria-label='搜索应用和命令']");
    await browser(
        "eval",
        "if (document.querySelector('.extension-view')) throw new Error('Explicit close retry did not leave view'); true"
    );
}

async function _verifyInvocationFailure(browser: BrowserCommand, output: string, hostService = false): Promise<void>
{
    const state = hostService ? "host-service-error" : "invocation-error";
    const english = hostService
        ? "Nanika could not confirm whether the history item was copied."
        : "Clipboard History could not be opened.";
    const chinese = hostService ? "Nanika 无法确认历史条目是否已复制" : "无法打开剪贴板历史";
    const detail = hostService ? "host service closed before replying" : "EACCES: /Example/历史.db";
    if (hostService)
    {
        await browser("set", "viewport", "460", "360");
    }
    await browser("eval", `location.search = '?surface=launcher&state=${state}'; true`);
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", "--fn", "document.documentElement.dataset.actionAttempts === '1'");
    await browser(
        "eval",
        `(() => {
        window.retainedActionRow = document.querySelector('[role=option][aria-selected=true]');
        window.retainedActionInput = document.querySelector('.launcher input');
        window.retainedActionReads = document.documentElement.dataset.localizationReads;
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "input[aria-label='搜索应用和命令']");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.actionAttempts !== '1') throw new Error('Language resubmitted accepted action');
        window.finishInvocationFailure(); return true;
    })()`
    );
    await browser("wait", "--text", chinese);
    await browser(
        "eval",
        `(() => {
        const row = document.querySelector('[role=option][aria-selected=true]');
        const input = document.querySelector('.launcher input');
        if (row !== window.retainedActionRow || input !== window.retainedActionInput || input.value !== 'Clipboard') throw new Error('Failure replaced search state');
        if (!document.querySelector('.operation-failure').textContent.includes(${
            JSON.stringify(detail)
        })) throw new Error('Action diagnostic lost');
        window.retainedActionError = document.querySelector('.operation-failure');
        window.changeLauncherLanguage('en'); return true;
    })()`
    );
    await browser("wait", "--text", english);
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.operation-failure') !== window.retainedActionError) throw new Error('Language replaced action error');
        if (document.documentElement.dataset.localizationReads !== window.retainedActionReads || document.documentElement.dataset.actionAttempts !== '1') throw new Error('Error translation performed backend work');
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "--text", chinese);
    await browser(
        "screenshot",
        join(
            output,
            hostService ? "launcher-host-service-unconfirmed-chinese.png" : "launcher-action-error-chinese.png"
        )
    );
    if (hostService)
    {
        await browser(
            "eval",
            `(() => {
            const failure = document.querySelector('.operation-failure');
            const bounds = failure.getBoundingClientRect();
            if (bounds.left < 0 || bounds.right > innerWidth || bounds.bottom > innerHeight || failure.scrollWidth > failure.clientWidth + 1) throw new Error('Host service failure overflows narrow launcher');
            if (!failure.textContent.includes('请先检查剪贴板内容')) throw new Error('Unconfirmed outcome lost recovery guidance');
            return true;
        })()`
        );
    }
    await browser("focus", "input[aria-label='搜索应用和命令']");
    await browser("press", "Enter");
    await browser("wait", ".extension-view");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.actionAttempts !== '2' || document.querySelector('.operation-failure')) throw new Error('Explicit retry did not recover');
        return true;
    })()`
    );
    if (hostService)
    {
        await browser("set", "viewport", "920", "680");
    }
}

async function _verifyTextReadFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=text-read-error'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", ".text-document");
    await browser(
        "eval",
        "document.querySelector('.detail-pane .scroll-viewport').scrollTop = document.querySelector('.detail-pane .scroll-viewport').scrollHeight; true"
    );
    await browser("wait", "--text", "Clipboard text could not be loaded.");
    await browser(
        "eval",
        `(() => {
        window.retainedText = document.querySelector('.text-document');
        window.retainedTextContent = window.retainedText.textContent;
        window.retainedText.focus({preventScroll:true});
        const walker = document.createTreeWalker(window.retainedText, NodeFilter.SHOW_TEXT);
        let node = walker.nextNode(); while (node && node.textContent.length < 12) node = walker.nextNode();
        if (!node) throw new Error('Missing source text node');
        const range = document.createRange(); range.setStart(node, 2); range.setEnd(node, 12);
        getSelection().removeAllRanges(); getSelection().addRange(range);
        window.retainedTextSelection = getSelection().toString();
        window.retainedTextScroll = document.querySelector('.detail-pane .scroll-viewport').scrollTop;
        window.retainedTextReads = document.documentElement.dataset.textReads;
        window.retainedResourceReads = document.documentElement.dataset.localizationReads;
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "--text", "无法加载剪贴板文本");
    await browser(
        "eval",
        `(() => {
        const text = document.querySelector('.text-document');
        if (text !== window.retainedText || text.textContent !== window.retainedTextContent || document.activeElement !== text || getSelection().toString() !== window.retainedTextSelection) throw new Error('Error translation reset native text state');
        if (Math.abs(document.querySelector('.detail-pane .scroll-viewport').scrollTop - window.retainedTextScroll) > 1) throw new Error('Error translation reset scroll');
        if (document.documentElement.dataset.textReads !== window.retainedTextReads || document.documentElement.dataset.localizationReads !== window.retainedResourceReads) throw new Error('Error translation performed backend work');
        if (!document.querySelector('.text-preview [role=status]').textContent.includes('EACCES: /Example/私密文件.txt')) throw new Error('Original diagnostic was lost');
        return true;
    })()`
    );
    await browser(
        "eval",
        "document.querySelector('.text-preview [role=status]').scrollIntoView({block:'end'}); true"
    );
    await browser("screenshot", join(output, "text-read-error-chinese.png"));
    await browser("eval", "window.allowTextRead(); true");
    await browser("find", "role", "button", "click", "--name", "重试", "--exact");
    await browser("wait", "--fn", "!document.querySelector('.text-preview [role=status]')");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.text-document') !== window.retainedText || window.retainedText.textContent !== window.retainedTextContent + 'B') throw new Error('Retry replaced resident text');
        if (Number(document.documentElement.dataset.textReads) !== Number(window.retainedTextReads) + 1) throw new Error('Retry did not issue exactly one read');
        return true;
    })()`
    );
}

async function _verifyLocalizedList(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=localized-list'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", "input[placeholder='Search clipboard history']");
    await browser(
        "eval",
        `(() => {
        const text = document.querySelector('.extension-view').textContent.replace(/[\\u2068\\u2069]/g, '');
        if (!text.includes('Image 1920 × 1080') || !text.includes('报告.txt and 2 more')) throw new Error('Generated Clipboard labels did not format in English');
        return true;
    })()`
    );
    await browser("fill", "input[placeholder='Search clipboard history']", "retained query");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('.extension-view .content')?.getAttribute('aria-busy') === 'false'"
    );
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('.extension-view input');
        input.setSelectionRange(2, 6);
        const pane = document.querySelector('.extension-viewport');
        pane.scrollTop = 300;
        window.retainedView = { input, pane, row: document.querySelector('[role=option][aria-selected=true]'),
            document: document.querySelector('.text-preview'), filter: document.querySelector('.filter button[aria-pressed=true]'),
            reads: document.documentElement.dataset.localizationReads, events: document.documentElement.dataset.viewEventCount };
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "input[placeholder='搜索剪贴板历史']");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedView;
        const input = document.querySelector('.extension-view input');
        if (input !== kept.input || document.activeElement !== input || input.value !== 'retained query'
            || input.selectionStart !== 2 || input.selectionEnd !== 6) throw new Error('View input state changed with language');
        if (document.querySelector('[role=option][aria-selected=true]') !== kept.row
            || !kept.row.textContent.includes('文本') || !kept.row.textContent.includes('内容类型')) throw new Error('Selected row identity or labels changed incorrectly');
        if (document.querySelector('.extension-viewport') !== kept.pane || kept.pane.scrollTop !== 300) throw new Error('View scroll anchor changed with language');
        if (document.querySelector('.filter button[aria-pressed=true]') !== kept.filter || kept.filter.textContent.trim() !== '所有类型') throw new Error('Filter identity changed');
        if (!kept.document || document.querySelector('.text-preview') !== kept.document) throw new Error('Language change replaced the text document');
        if (!document.querySelector('.detail-pane').textContent.includes('内容类型')
            || !document.querySelector('.detail-pane').textContent.includes("const original = '内容';")
            || !document.querySelector('.detail-pane').textContent.includes('大小')
            || !document.querySelector('.detail-pane').textContent.includes('1.5 KiB')) throw new Error('Detail metadata or original document changed');
        if (document.documentElement.dataset.localizationReads !== kept.reads || document.documentElement.dataset.viewEventCount !== kept.events) throw new Error('Language change caused a resource read or view event');
        const text = document.querySelector('.extension-view').textContent.replace(/[\\u2068\\u2069]/g, '');
        if (!text.includes('图片 1920 × 1080') || !text.includes('报告.txt和另外 2 个文件')) throw new Error('Generated Clipboard facts did not translate');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("hover", ".extension-viewport [role=option][aria-selected=true]");
    await verifyScrollGutters(browser);
    await _verifyRowSpacing(browser, ".extension-viewport");
    await browser("screenshot", join(output, "extension-list-chinese.png"));
    await browser("eval", "document.querySelector('.extension-viewport').scrollTop = 0; true");
    await browser("screenshot", join(output, "clipboard-facts-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "input[placeholder='Search clipboard history']");
    await browser("screenshot", join(output, "clipboard-facts-english.png"));
}

async function _verifyConfirmationExit(browser: BrowserCommand, confirmed: boolean, reduced: boolean): Promise<void>
{
    const search = "input[aria-label='Search for apps and commands']";
    await browser("fill", search, "Empty");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "wait",
        "--fn",
        "Number(getComputedStyle(document.querySelector('[role=alertdialog]')).opacity) === 1"
    );
    await browser(
        "eval",
        `(() => {
        const root = document.documentElement;
        const search = document.querySelector("input[aria-label='Search for apps and commands']");
        const initialCount = Number(root.dataset.invocationCount ?? 0);
        delete root.dataset.dismissed;
        delete root.dataset.confirmationAtInvoke;
        window.confirmationExit = new Promise((resolve, reject) => {
            const frames = [];
            let exercised = false;
            const deadline = performance.now() + 3000;
            const sample = () => {
                try {
                    const dialog = document.querySelector('[role=alertdialog]');
                    const opacity = dialog ? Number(getComputedStyle(dialog).opacity) : 0;
                    frames.push(opacity);
                    if (dialog?.hasAttribute('data-ending-style')) {
                        if (!document.body.inert || document.activeElement === search) {
                            throw new Error('Confirmation released input before its exit completed');
                        }
                        if (${reduced} && getComputedStyle(dialog).transform !== 'none') {
                            throw new Error('Reduced motion retained the dialog scale');
                        }
                        if (!exercised && opacity > 0 && opacity < 1) {
                            exercised = true;
                            search.focus();
                            if (document.activeElement === search) throw new Error('Closing modal allowed search focus');
                            for (const key of ['x', 'Escape', 'Tab', 'Enter']) {
                                const event = new KeyboardEvent('keydown', { key, repeat: true, bubbles: true, cancelable: true });
                                if (document.activeElement.dispatchEvent(event)) throw new Error('Closing modal leaked ' + key);
                            }
                            // Synthetic clicks bypass inert; the recorded choice must still be immutable.
                            for (const button of dialog.querySelectorAll('button')) button.click();
                        }
                    }
                    if (opacity > 0 && (root.dataset.dismissed || root.dataset.confirmationAtInvoke)) {
                        throw new Error('Confirmation exit was cut short');
                    }
                    if (!dialog) {
                        const count = Number(root.dataset.invocationCount ?? 0) - initialCount;
                        const receipt = root.dataset.confirmationAtInvoke ? JSON.parse(root.dataset.confirmationAtInvoke) : null;
                        if (!exercised || document.body.inert || document.activeElement !== search || search.value !== 'Empty') {
                            throw new Error('Confirmation did not restore unchanged search after the complete exit');
                        }
                        if (count !== ${confirmed ? 1 : 0} || Boolean(root.dataset.dismissed) !== ${confirmed}
                            || (${confirmed} && (!receipt?.confirmed || receipt.opacity !== 0))) {
                            throw new Error('Confirmation lost or changed its recorded decision');
                        }
                        resolve({ confirmed: ${confirmed}, reduced: ${reduced}, frames, receipt, count });
                    } else if (performance.now() > deadline) throw new Error('Confirmation did not finish');
                    else requestAnimationFrame(sample);
                } catch (error) { reject(error); }
            };
            requestAnimationFrame(sample);
        });
        return true;
    })()`
    );
    await browser("press", confirmed ? "Enter" : "Escape");
    await browser("eval", "window.confirmationExit");
}

async function _verifyConfirmationInterruption(browser: BrowserCommand): Promise<void>
{
    await browser("fill", "input[aria-label='Search for apps and commands']", "Empty");
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "wait",
        "--fn",
        "Number(getComputedStyle(document.querySelector('[role=alertdialog]')).opacity) === 1"
    );
    await browser(
        "eval",
        `(async () => {
        const root = document.documentElement;
        const initialCount = Number(root.dataset.invocationCount ?? 0);
        const dialog = document.querySelector('[role=alertdialog]');
        [...dialog.querySelectorAll('button')].find(button => button.textContent.trim() === 'Empty Trash').click();
        if (!document.body.inert) throw new Error('Confirmation did not acquire input isolation');
        await new Promise(requestAnimationFrame);
        window.dispatchEvent(new Event('blur'));
        if (document.body.inert) throw new Error('Interrupted confirmation retained input isolation');
        if (document.activeElement.matches('input')) throw new Error('Interrupted confirmation restored focus');
        await new Promise(requestAnimationFrame);
        if (Number(root.dataset.invocationCount) !== initialCount + 1) {
            throw new Error('Interrupted confirmation lost its accepted invocation');
        }
        return true;
    })()`
    );
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser(
        "eval",
        "(() => { if (document.body.inert || document.activeElement.matches('input')) throw new Error('Stale exit restored focus after interruption'); return true; })()"
    );
}

async function _verifyConfigurationFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "window.failNextConfigurationSave()");
    await browser("find", "role", "menuitemcheckbox", "click", "--name", "Syntax highlighting", "--exact");
    await browser("wait", "--text", "The change could not be saved and was not applied.");
    await browser(
        "eval",
        `(() => {
        window.retainedConfigurationError = document.querySelector('[role=alert]');
        window.retainedConfigurationContent = document.querySelector('article');
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "--text", "更改无法保存，尚未应用。");
    await browser(
        "eval",
        `(() => {
        const alert = document.querySelector('[role=alert]');
        if (alert !== window.retainedConfigurationError || !alert.textContent.includes('fixture disk is read-only')) throw new Error('Language changed the operation error or diagnostic');
        if (!window.retainedConfigurationContent || document.querySelector('article') !== window.retainedConfigurationContent) throw new Error('Error translation replaced the view');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("screenshot", join(output, "configuration-error-chinese.png"));
    await browser("eval", "window.changeLauncherLanguage('en')");
    await browser("wait", "--text", "The change could not be saved and was not applied.");
    await browser("wait", "--fn", "!document.querySelector('[role=menu]')");
    await browser("find", "role", "button", "click", "--name", "\u2068Clipboard History\u2069 menu", "--exact");
    await browser("wait", "[role=menuitemcheckbox][aria-checked=false]");
}

async function _verifyLocalizedFiles(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=localized-files'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", ".file-paths");
    await browser(
        "eval",
        `(() => {
        const detail = document.querySelector('.detail-pane');
        window.retainedFiles = { detail, paths: [...detail.querySelectorAll('.file-paths span')].slice(0, 5).map(node => [node, node.textContent]),
            reads: document.documentElement.dataset.localizationReads, events: document.documentElement.dataset.viewEventCount };
        if (!detail.querySelector('[aria-label="File collection"]') || !detail.textContent.includes('more files')) throw new Error('English file collection labels missing');
        window.changeLauncherLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "[aria-label='文件集合']");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedFiles;
        const detail = document.querySelector('.detail-pane');
        const plain = detail.textContent.replace(/[\\u2068\\u2069]/g, '');
        if (detail !== kept.detail || !plain.includes('7 个项目') || !plain.includes('还有 2 个文件') || !plain.includes('路径')) throw new Error('File facts did not translate in place');
        for (const [node, value] of kept.paths) if (!node.isConnected || node.textContent !== value) throw new Error('Language switching rewrote a file path');
        if (document.documentElement.dataset.localizationReads !== kept.reads || document.documentElement.dataset.viewEventCount !== kept.events) throw new Error('File presentation change performed business work');
        if (document.documentElement.scrollWidth > innerWidth) throw new Error('File collection overflowed');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "file-collection-chinese.png"));
}

async function _verifyStartupFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=startup-failure'; true");
    await browser("wait", "--text", "Nanika could not open or update its database.");
    await browser(
        "eval",
        `(() => {
        if (document.body.textContent.includes('SQLITE_CANTOPEN') || document.documentElement.dataset.startupDiagnosticReads) throw new Error('Startup leaked diagnostics before explicit request');
        if ([...document.querySelectorAll('button')].some(button => button.textContent.includes('Try again'))) throw new Error('WebView reload cannot retry runtime startup');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-startup-failure-english.png"));
    await browser("find", "role", "button", "click", "--name", "Show diagnostics", "--exact");
    await browser("wait", "textarea[aria-label='Startup diagnostics']");
    await browser(
        "eval",
        `(() => {
        const diagnostic = document.querySelector('#startup-diagnostic textarea');
        diagnostic.focus(); diagnostic.scrollTop = 120;
        diagnostic.setSelectionRange(0, 15);
        window.retainedStartup = { diagnostic, text: diagnostic.value, scroll: diagnostic.scrollTop, selection: diagnostic.value.slice(diagnostic.selectionStart, diagnostic.selectionEnd), queries: document.documentElement.dataset.queryCount, resources: document.documentElement.dataset.localizationReads };
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "--text", "Nanika 无法打开或更新数据库。");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedStartup; const diagnostic = document.querySelector('#startup-diagnostic textarea');
        if (diagnostic !== kept.diagnostic || diagnostic.value !== kept.text) throw new Error('Language replaced startup diagnostics');
        if (document.activeElement !== diagnostic || diagnostic.value.slice(diagnostic.selectionStart, diagnostic.selectionEnd) !== kept.selection || diagnostic.scrollTop !== kept.scroll) throw new Error('Language moved diagnostic focus, selection or scroll');
        if (document.documentElement.dataset.startupDiagnosticReads !== '1' || document.documentElement.dataset.queryCount !== kept.queries || document.documentElement.dataset.localizationReads !== kept.resources) throw new Error('Language retried startup or fetched data');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-startup-diagnostic-chinese.png"));
    await browser("set", "viewport", "460", "360");
    await browser(
        "eval",
        `(() => {
        const pane = document.querySelector('.launcher-failure');
        if (pane.scrollWidth > pane.clientWidth + 1) throw new Error('Startup presentation overflows narrow window');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-startup-narrow.png"));
    await browser("find", "role", "button", "click", "--name", "隐藏诊断", "--exact");
    await browser("find", "role", "button", "click", "--name", "查看诊断", "--exact");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.startupDiagnosticReads !== '1') throw new Error('Reopening refetched retained diagnostic');
        window.changeLauncherLanguage('en'); return true;
    })()`
    );
    await browser("wait", "--text", "Nanika could not open or update its database.");
    await browser("set", "viewport", "920", "680");
}

async function _verifyStartupDiagnosticFailure(browser: BrowserCommand): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=startup-diagnostic-error'; true");
    await browser("wait", "--text", "Nanika could not open or update its database.");
    await browser("find", "role", "button", "click", "--name", "Show diagnostics", "--exact");
    await browser("wait", "--text", "Startup diagnostics are unavailable.");
    await browser("eval", "window.changeLauncherLanguage('zh-Hans'); true");
    await browser("wait", "--text", "启动诊断不可用。");
    await browser(
        "eval",
        `(() => {
        if (!document.querySelector('.explanation').textContent.includes('Nanika 无法打开或更新数据库。')) throw new Error('Diagnostic read replaced the original startup cause');
        if (document.documentElement.dataset.startupDiagnosticReads !== '1') throw new Error('Language retried the failed diagnostic read');
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "隐藏诊断", "--exact");
    await browser("find", "role", "button", "click", "--name", "查看诊断", "--exact");
    await browser("wait", "textarea[aria-label='启动诊断']");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.startupDiagnosticReads !== '2' || !document.querySelector('textarea').value.includes('SQLITE_CANTOPEN')) throw new Error('Explicit diagnostic retry did not recover');
        return true;
    })()`
    );
}

async function _verifyIsolatedTitle(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=isolated-title'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "shortcut42");
    await browser("wait", "[role=option] mark");
    await browser(
        "eval",
        `(() => {
        const title = document.querySelector('[role=option] .title');
        if (title.textContent !== 'shortcut\\u206842\\u2069') throw new Error('Highlighting rewrote bidi output');
        if ([...title.querySelectorAll('mark')].map(mark => mark.textContent).join('').replace(/[\\u2066-\\u2069]/gu, '') !== 'shortcut42') throw new Error('Isolates split the highlighted token');
        window.retainedIsolatedTitle = title;
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "input[aria-label='搜索应用和命令']");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('[role=option] .title') !== window.retainedIsolatedTitle) throw new Error('Language replaced literal content');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-isolated-title.png"));
}

async function _verifyQueryFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=query-error'; true");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "app");
    await browser("wait", "--text", "Search could not be updated.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input');
        input.focus(); input.setSelectionRange(1, 2);
        window.queryFailureInput = input;
        window.queryFailureNode = document.querySelector('#query-error');
        window.failedQueryCount = document.documentElement.dataset.queryCount;
        window.changeLauncherLanguage('zh-Hans'); return true;
    })()`
    );
    await browser("wait", "--text", "无法更新搜索");
    await browser(
        "eval",
        `(() => {
        const input = window.queryFailureInput;
        if (document.activeElement !== input || input.value !== 'app' || input.selectionStart !== 1 || input.selectionEnd !== 2) throw new Error('Query failure lost input state');
        if (document.querySelector('#query-error') !== window.queryFailureNode || !window.queryFailureNode.textContent.includes('Original query <diagnostic>')) throw new Error('Query diagnostic was replaced');
        if (document.documentElement.dataset.queryCount !== window.failedQueryCount) throw new Error('Language resubmitted the query');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "launcher-query-error-chinese.png"));
    await browser("fill", "input[aria-label='搜索应用和命令']", "apple");
    await browser("wait", "--fn", "!document.querySelector('#query-error')");
    await browser("fill", "input[aria-label='搜索应用和命令']", "界".repeat(1025));
    await browser("wait", "--text", "搜索最多支持");
    await browser(
        "eval",
        `(() => {
        window.queryFailureInput = document.querySelector('input');
        window.queryFailureInput.setSelectionRange(1020, 1025);
        window.invalidQueryCount = document.documentElement.dataset.queryCount;
        window.changeLauncherLanguage('en'); return true;
    })()`
    );
    await browser("wait", "--text", "Search supports up to");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input');
        if (input !== window.queryFailureInput || input.value !== '界'.repeat(1025) || input.selectionStart !== 1020 || input.selectionEnd !== 1025) throw new Error('Length error changed the invalid draft');
        if (document.documentElement.dataset.maximumQueryLength !== '5') throw new Error('Oversized input reached the backend');
        if (document.documentElement.dataset.queryCount !== window.invalidQueryCount) throw new Error('Language resubmitted invalid input');
        if (!document.querySelector('#query-error').textContent.includes('1024')) throw new Error('Length error lost its bound');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
}

async function _verifyLongLabels(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=long-labels'; true");
    await browser("set", "viewport", "460", "360");
    await browser("wait", "input[aria-label='Search for apps and commands']");
    await browser("fill", "input[aria-label='Search for apps and commands']", "Clipboard");
    await browser("wait", ".results[aria-busy=false] [role=option][aria-selected=true]");
    await browser("press", "Enter");
    await browser("wait", ".extension-view .filter");
    for (const locale of ["en", "zh-Hans"])
    {
        await browser("eval", `window.changeLauncherLanguage('${locale}')`);
        await browser("wait", "--fn", `document.documentElement.lang === '${locale}'`);
        await browser(
            "eval",
            `(() => {
            const header = document.querySelector('.extension-view header');
            const query = header.querySelector('input');
            const filter = header.querySelector('.filter');
            const bad = [...document.querySelectorAll('.detail-pane dt,.detail-pane dd')]
                .filter(element => element.scrollWidth > element.clientWidth + 1)
                .map(element => ({tag:element.tagName, width:element.clientWidth, scroll:element.scrollWidth}));
            if (query.clientWidth < 120 || filter.getBoundingClientRect().right > innerWidth || bad.length)
                throw new Error('Long translated view text displaced content: ' + JSON.stringify({query:query.clientWidth, filter:filter.getBoundingClientRect().toJSON(), bad}));
            return true;
        })()`
        );
        await browser("screenshot", join(output, `view-long-labels-${locale}.png`));
        await browser("click", ".filter button:last-child");
        await browser("wait", ".filter button:last-child[aria-pressed=true]");
        await browser("hover", ".extension-view header input");
        await browser("hover", ".filter button:last-child");
        await browser("wait", "[role=tooltip]");
        await browser(
            "eval",
            `(() => {
            const button = document.querySelector('.filter button:last-child');
            const tooltip = document.querySelector('[role=tooltip]');
            const bounds = tooltip.getBoundingClientRect();
            if (tooltip.textContent.trim() !== button.textContent.trim() || bounds.left < 0 || bounds.right > innerWidth)
                throw new Error('Truncated filter did not expose its full label within the window');
            return true;
        })()`
        );
        await browser("press", "Escape");
        await browser("wait", "--fn", "!document.querySelector('[role=tooltip]')");
        await browser("click", ".filter button:first-child");
        await browser("wait", ".filter button:first-child[aria-pressed=true]");
    }
    await browser("eval", "location.search = '?surface=launcher'; true");
    await browser("set", "viewport", "920", "680");
    await browser("wait", "input[aria-label='Search for apps and commands']");
}

async function _verifyScrollLayout(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=launcher&state=scroll-layout'; true");
    await browser("set", "viewport", "460", "360");
    await browser("wait", ".root-viewport [role=option]");
    await browser("hover", ".root-viewport [role=option][aria-selected=true]");
    await browser(
        "eval",
        `(() => {
        const viewport = document.querySelector('.root-viewport');
        const area = viewport.closest('.scroll-area');
        const track = area.querySelector(':scope > .scroll-track');
        const row = viewport.querySelector('[role=option][aria-selected=true]');
        const edge = track.getBoundingClientRect().left;
        if (viewport.scrollHeight <= viewport.clientHeight) throw new Error('Root scrollbar fixture does not overflow');
        if (row.getBoundingClientRect().right > edge - 3) throw new Error('Root selection overlaps scrollbar: ' + JSON.stringify({row:row.getBoundingClientRect().toJSON(), track:track.getBoundingClientRect().toJSON()}));
        window.scrollLayout = {viewport, row, width:row.getBoundingClientRect().width};
        return true;
    })()`
    );
    await verifyScrollGutters(browser);
    await _verifyRowSpacing(browser, ".root-viewport");
    await browser("screenshot", join(output, "root-scrollbar-gutter.png"));
    await browser("hover", ".collection-header input");
    await browser(
        "wait",
        "--fn",
        "getComputedStyle(document.querySelector('.root-viewport').closest('.scroll-area').querySelector('.scroll-track')).opacity === '0'"
    );
    await browser(
        "eval",
        `(() => {
        const state = window.scrollLayout;
        if (state.row.getBoundingClientRect().width !== state.width) throw new Error('Scrollbar visibility moved content');
        state.viewport.scrollTop = 100;
        return true;
    })()`
    );
    await browser("wait", ".scroll-area[data-revealed]");
    await browser("drag", ".results .scroll-thumb", ".results .scroll-track");
    await browser(
        "eval",
        `(() => {
        const state = window.scrollLayout;
        if (state.viewport.scrollTop <= 100) throw new Error('Dragging the reserved scrollbar did not scroll');
        if (!document.activeElement.matches('.collection-header input')) throw new Error('Scrollbar drag stole search focus');
        return true;
    })()`
    );
    await browser("fill", ".collection-header input", "Application 29");
    await browser("wait", "--fn", "document.querySelectorAll('.root-viewport [role=option]').length === 1");
    await browser(
        "eval",
        `(() => {
        const state = window.scrollLayout;
        if (document.querySelector('.root-viewport') !== state.viewport || state.viewport.scrollHeight > state.viewport.clientHeight)
            throw new Error('Short results did not reuse the viewport without overflow');
        if (document.querySelector('.root-viewport [role=option]').getBoundingClientRect().width !== state.width)
            throw new Error('Overflow removal changed result width');
        return true;
    })()`
    );
    await verifyScrollGutters(browser);
    await browser("eval", "location.search = '?surface=launcher'; true");
    await browser("set", "viewport", "920", "680");
    await browser("wait", "input[aria-label='Search for apps and commands']");
}

async function _verifyRowSpacing(browser: BrowserCommand, selector: string): Promise<void>
{
    await browser("hover", `${selector} [role=option][aria-posinset='2']`);
    await browser(
        "eval",
        `(() => {
        const items = [...document.querySelectorAll('${selector} [role=option]')].slice(0, 2);
        if (items.length !== 2 || items[0].getAttribute('aria-selected') !== 'true' || !items[1].matches(':hover'))
            throw new Error('Spacing fixture needs adjacent selected and hovered rows');
        const rows = items.map(item => item.matches('.collection-row') ? item : item.querySelector('.collection-row'));
        const geometry = rows.map(row => {
            const rect = row.getBoundingClientRect();
            const style = getComputedStyle(row);
            const icon = row.querySelector('.collection-icon').getBoundingClientRect();
            if (style.backgroundClip !== 'padding-box' || style.backgroundColor === 'rgba(0, 0, 0, 0)')
                throw new Error('Selected/hovered background did not respect the row spacing');
            return {top:rect.top, height:rect.height, paintTop:rect.top+parseFloat(style.borderTopWidth),
                paintBottom:rect.bottom-parseFloat(style.borderBottomWidth), iconX:icon.left-rect.left,
                iconCenter:icon.top+icon.height/2-rect.top};
        });
        const [first, second] = geometry;
        if (geometry.some(row => row.height !== 52 || row.iconCenter !== 26 || row.iconX !== 12)
            || second.top-first.top !== 52 || second.paintTop-first.paintBottom !== 4)
            throw new Error('Collection row geometry changed: ' + JSON.stringify(geometry));
        return geometry;
    })()`
    );
}

async function _verifyActionShortcuts(browser: BrowserCommand, output: string): Promise<void>
{
    const search = "input[aria-label='Search for apps and commands']";
    for (const platform of ["macos", "windows"])
    {
        await browser("eval", `location.search = '?surface=launcher&state=shortcuts&platform=${platform}'`);
        await browser("wait", search);
        await browser("fill", search, "long application");
        await browser("wait", "[role=option][aria-selected=true]");
        await browser(
            "eval",
            `(() => {
            window.shortcutEvent = { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true, cancelable: true,
                ${platform === "macos" ? "metaKey" : "ctrlKey"}: true };
            const input = document.querySelector('[role=combobox]');
            for (const patch of [{repeat:true}, {isComposing:true}, {keyCode:229}, {altKey:true},
                {metaKey:${platform !== "macos"}, ctrlKey:${platform !== "windows"}}]) {
                input.dispatchEvent(new KeyboardEvent('keydown', {...window.shortcutEvent, ...patch}));
            }
            if (document.documentElement.dataset.shortcutInvocations) throw new Error('Unrelated or repeated key invoked an action');
            document.documentElement.dataset.shortcutPaused = 'true';
            for (let i = 0; i < 3; i++) input.dispatchEvent(new KeyboardEvent('keydown', window.shortcutEvent));
            if (document.documentElement.dataset.shortcutInvocations !== '1') throw new Error('Repeated input was not bounded');
            document.documentElement.dataset.shortcutPaused = 'false';
        })()`
        );
        await browser("wait", "--fn", "document.querySelector('.results').getAttribute('aria-busy') === 'false'");
        await browser(
            "eval",
            `(() => {
            const request = JSON.parse(document.documentElement.dataset.shortcutRequest);
            if (request.extensionId !== 'external.shortcuts' || request.actionId !== 'custom.location'
                || request.invocation !== 'explicit') throw new Error('Host did not forward the declared external action');
            if (document.querySelector('[role=menu]')) throw new Error('Shortcut flashed a menu');
        })()`
        );
        await _dispatchActionShortcut(browser, { key: "K", code: "KeyK", keyCode: 75, shiftKey: true });
        await browser(
            "wait",
            "--fn",
            "JSON.parse(document.documentElement.dataset.shortcutRequest).actionId === 'custom.inspect'"
        );
        await _dispatchActionShortcut(browser, { key: "9", code: "Digit9", keyCode: 57 });
        await browser(
            "eval",
            "if (document.documentElement.dataset.shortcutInvocations !== '2') throw new Error('Disabled action was invoked');"
        );
        await browser("hover", "[role=option][aria-selected=true]");
        await browser("mouse", "down", "right");
        await browser("mouse", "up", "right");
        await browser("wait", "[role=menuitem]");
        await browser("wait", "--fn", "getComputedStyle(document.querySelector('[role=menu]')).opacity === '1'");
        await browser(
            "eval",
            `(() => {
            const menu = document.querySelector('[role=menu]');
            const inspect = [...menu.querySelectorAll('[role=menuitem]')].find(item => item.textContent.includes('Inspect'));
            if (inspect?.querySelector('.shortcut-keys kbd:last-child')?.textContent.trim() !== 'T') {
                throw new Error('Physical shortcut label did not use the native layout');
            }
            for (const label of menu.querySelectorAll('kbd .key-label')) {
                if (label.getBoundingClientRect().height > parseFloat(getComputedStyle(label).lineHeight) + 1) throw new Error('Shortcut key wrapped');
            }
            if (menu.scrollWidth > menu.clientWidth) throw new Error('Shortcut menu overflowed');
            if (!menu.textContent.includes('${
                platform === "macos" ? "⌘" : "Ctrl"
            }') || !menu.querySelector('kbd svg')) throw new Error('Menu omitted shortcut hints');
        })()`
        );
        await browser("screenshot", join(output, `action-shortcuts-${platform}.png`));
        await browser("press", "Escape");
        await browser("wait", "--fn", "!document.querySelector('[role=menu]')");
        await _dispatchActionShortcut(browser, { shiftKey: true });
        await browser("wait", "[role=alertdialog]");
        await browser("press", "Escape");
        await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
        await browser(
            "eval",
            "if (document.documentElement.dataset.shortcutInvocations !== '2') throw new Error('Cancelled action was invoked');"
        );
        await _dispatchActionShortcut(browser, { shiftKey: true });
        await browser("wait", "[role=alertdialog]");
        await browser("find", "role", "button", "click", "--name", "Confirm action", "--exact");
        await browser(
            "wait",
            "--fn",
            "JSON.parse(document.documentElement.dataset.shortcutRequest).invocation === 'confirmed'"
        );
        await browser("fill", search, "Clipboard");
        await browser("wait", "--text", "Clipboard History");
        await _dispatchActionShortcut(browser);
        await browser(
            "eval",
            "if (document.documentElement.dataset.shortcutInvocations !== '3' || !document.querySelector('.launcher')) throw new Error('Item without a binding reacted');"
        );
        await browser("press", "Enter");
        await browser("wait", "--text", "Code preview");
        await browser(
            "eval",
            `(() => {
                document.activeElement.dispatchEvent(new KeyboardEvent('keydown', window.shortcutEvent));
                if (document.documentElement.dataset.shortcutInvocations !== '3') throw new Error('Root shortcut leaked into extension view');
            })()`
        );
        await browser("press", "Escape");
        await browser("wait", search);
        await browser("fill", search, "long application");
        await browser("wait", "[role=option][aria-selected=true]");
        await browser("eval", "document.documentElement.dataset.shortcutFailure = 'true'");
        await _dispatchActionShortcut(browser);
        await browser("wait", ".operation-failure");
        await browser(
            "eval",
            "if (document.querySelector('[role=combobox]').value !== 'long application') throw new Error('Action failure discarded query');"
        );
    }
}

/**
 * Exercise cross-platform UI handling without invoking browser or OS menu accelerators.
 * Real shortcut input is verified by the native application E2E.
 *
 * @param browser The owned browser session
 * @param changes Key identity and modifier changes relative to the fixture's primary+Enter event
 */
async function _dispatchActionShortcut(
    browser: BrowserCommand,
    changes: { code?: string; key?: string; keyCode?: number; shiftKey?: boolean; } = {}
): Promise<void>
{
    await browser(
        "eval",
        `document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {
            ...window.shortcutEvent, ...${JSON.stringify(changes)}
        }))`
    );
}
