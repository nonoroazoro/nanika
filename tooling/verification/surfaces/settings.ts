import { join } from "node:path";

import { verifyScrollGutters } from "../scroll-area.ts";

import type { BrowserCommand } from "../BrowserCommand.ts";

/**
 * Exercises the real settings surface with deterministic bridge fixtures.
 *
 * @param browser Owned headless session
 * @param output Screenshot directory
 */
export async function verifySettings(browser: BrowserCommand, output: string): Promise<void>
{
    await _verifyScrollLayout(browser, output);
    await _verifyLanguageLayout(browser, output);
    await _verifyLongSettingsText(browser, output);
    await _verifyLifecycle(browser, output);
    await browser("fill", "input[aria-label='Maximum history entries']", "75");
    await browser(
        "find",
        "role",
        "button",
        "click",
        "--name",
        "\u2068Maximum history entries\u2069: Unlimited",
        "--exact"
    );
    await browser(
        "wait",
        "--fn",
        "document.querySelector('input[aria-label=\"Maximum history entries\"]').readOnly"
    );
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Maximum history entries"]');
        if (input.value !== '75') throw new Error('Unlimited discarded the configured value');
        return true;
    })()`
    );
    await browser(
        "find",
        "role",
        "button",
        "click",
        "--name",
        "\u2068Maximum history entries\u2069: Use limit \u206875\u2069",
        "--exact"
    );
    await browser(
        "wait",
        "--fn",
        "!document.querySelector('input[aria-label=\"Maximum history entries\"]').readOnly"
    );
    await browser("find", "role", "button", "click", "--name", "Add folder", "--exact");
    await browser("wait", "--text", "/Example/Added folder");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "eval",
        `(() => {
        const confirm = document.activeElement;
        if (!confirm?.textContent?.includes('Clear history')) throw new Error('Confirm must own initial focus');
        const frame = confirm.closest('.button-focus-frame');
        if (!frame || getComputedStyle(frame).outlineWidth !== '1px') throw new Error('Confirm needs its thin focus frame');
        return { focused: confirm.textContent, outline: getComputedStyle(frame).outlineWidth };
    })()`
    );
    await browser("screenshot", join(output, "confirmation.png"));
    await browser(
        "eval",
        `(async () => {
        window.retainedConfirmation = document.querySelector('[role=alertdialog]');
        window.retainedConfirmationFocus = document.activeElement;
        window.retainedConfirmationReads = document.documentElement.dataset.localizationReads;
        await window.changeSettingsLanguage('zh-Hans');
        return true;
    })()`
    );
    await browser("wait", "--text", "清除全部历史记录？");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('[role=alertdialog]') !== window.retainedConfirmation) throw new Error('Language replaced the confirmation');
        if (document.activeElement !== window.retainedConfirmationFocus || !document.activeElement.textContent.includes('清除历史记录')) throw new Error('Language changed confirmation focus or left an untranslated action');
        if (document.documentElement.dataset.localizationReads !== window.retainedConfirmationReads) throw new Error('Confirmation language switching fetched resources');
        if (!window.retainedConfirmation.textContent.includes('此操作无法撤销')) throw new Error('Confirmation consequences did not translate');
        if (![...window.retainedConfirmation.querySelectorAll('button')].some(button => button.textContent.trim() === '取消')) throw new Error('Confirmation cancel did not translate');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "confirmation-chinese.png"));
    await browser("eval", "window.changeSettingsLanguage('en')");
    await browser("wait", "--text", "Clear all history?");
    await browser("hover", ".confirmation-actions .button-focus-frame button");
    await browser("mouse", "down");
    await browser(
        "wait",
        "--fn",
        "new DOMMatrix(getComputedStyle(document.querySelector('.button-focus-frame button')).transform).a < 0.99"
    );
    await browser(
        "eval",
        `(() => {
        const frame = document.querySelector('.button-focus-frame');
        if (getComputedStyle(frame).transform !== 'none') throw new Error('The focus frame must not shrink');
        if (getComputedStyle(frame).outlineWidth !== '1px') throw new Error('Pressing must preserve the focus frame');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "confirmation-pressed.png"));
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "up");
    await browser(
        "eval",
        "(() => { if (document.documentElement.dataset.confirmed) throw new Error('Cancelled press confirmed the action'); return true; })()"
    );
    await browser(
        "eval",
        `(() => {
        const dialog = document.querySelector('[role=alertdialog]');
        const frozen = dialog.textContent;
        const deadline = performance.now() + 5000;
        let changed = false;
        let observed = false;
        window.confirmationLocaleExit = new Promise((resolve, reject) => {
            const sample = () => {
                try {
                    if (!dialog.isConnected) {
                        if (!observed) throw new Error('Locale change was not observed during confirmation exit');
                        resolve(true);
                        return;
                    }
                    if (dialog.hasAttribute('data-ending-style')) {
                        if (!changed) { changed = true; window.changeSettingsLanguage('zh-Hans').catch(reject); }
                        observed ||= document.documentElement.lang === 'zh-Hans';
                        if (dialog.textContent !== frozen) throw new Error('Closing confirmation changed its captured text');
                    }
                    if (performance.now() > deadline) throw new Error('Confirmation exit did not finish');
                    requestAnimationFrame(sample);
                } catch (error) { reject(error); }
            };
            requestAnimationFrame(sample);
        });
        return true;
    })()`
    );
    await browser("press", "Escape");
    await browser("eval", "window.confirmationLocaleExit");
    await browser("eval", "window.changeSettingsLanguage('en')");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser(
        "eval",
        "(() => { if (document.activeElement?.textContent?.trim() !== 'Clear') throw new Error('Dialog did not restore trigger focus'); return true; })()"
    );
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "down");
    await browser("mouse", "up");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("press", "Enter");
    await browser("wait", "--fn", "document.documentElement.dataset.confirmed === 'true'");
    await browser("fill", ".sidebar input", "clip");
    await browser("snapshot", "-i");
    await browser("screenshot", join(output, "search-results.png"));
    await browser("set", "media", "dark", "reduced-motion");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("hover", ".confirmation-actions .button-focus-frame button");
    await browser("mouse", "down");
    await browser(
        "eval",
        `(() => {
        if (!matchMedia('(prefers-reduced-motion: reduce)').matches) throw new Error('Reduced motion is not active');
        for (const element of document.querySelectorAll('.confirmation-dialog, .button-focus-frame button')) {
            if (getComputedStyle(element).transform !== 'none') throw new Error('Reduced motion still changes geometry');
        }
        return true;
    })()`
    );
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "up");
    await browser("press", "Escape");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser("set", "media", "dark");
    await browser("set", "viewport", "700", "500");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.scrollWidth > innerWidth) throw new Error('Settings overflows its minimum window width');
        const content = document.querySelector('[aria-label="Settings content"]');
        if (!content || content.getBoundingClientRect().right > innerWidth) throw new Error('Settings content extends outside the window');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-minimum-size.png"));
    await browser("fill", ".sidebar input", "");
    await browser("find", "role", "button", "click", "--name", "General", "--exact");
    await browser("wait", "[role=combobox][aria-label='Language']");
    await browser(
        "eval",
        `(() => {
        window.languageControl = document.querySelector('[role=combobox][aria-label="Language"]');
        window.generalHeading = document.querySelector('.general-page h1');
        return true;
    })()`
    );
    await browser("find", "role", "combobox", "click", "--name", "Language", "--exact");
    await browser("find", "role", "option", "click", "--name", "简体中文", "--exact");
    await browser("wait", "--fn", "document.documentElement.lang === 'zh-Hans'");
    await browser("wait", "--fn", "!document.querySelector('[role=listbox]')");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.general-page h1') !== window.generalHeading) throw new Error('Locale remounted the page');
        if (window.generalHeading.textContent !== '通用') throw new Error('General heading was not translated');
        const search = document.querySelector('.sidebar input');
        if (search.getAttribute('aria-label') !== '搜索设置' || search.placeholder !== '搜索') throw new Error('Settings search labels did not translate');
        const control = document.querySelector('[role=combobox][aria-label="语言"]');
        if (control !== window.languageControl) throw new Error('Locale recreated the control');
        if (document.activeElement !== control) throw new Error('Language selection lost focus');
        if (document.documentElement.scrollWidth > innerWidth) throw new Error('Chinese settings overflow');
        if (![...document.querySelectorAll('.sidebar button')].some(button => button.textContent.trim() === '剪贴板历史')) {
            throw new Error('The package name did not translate with the host');
        }
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-chinese-minimum-size.png"));
    await _verifyAboutSearch(browser, output);
    await browser("find", "role", "button", "click", "--name", "剪贴板历史", "--exact");
    await browser("wait", "--fn", "document.querySelector('.extension-heading h1')?.textContent === '剪贴板历史'");
    await browser("fill", ".sidebar input", "clip");
    await browser("wait", ".result-group");
    await browser(
        "eval",
        `(() => {
        if (!document.querySelector('.result-group h2').textContent.includes('剪贴板历史')) throw new Error('Search lost the package resource scope');
        if (![...document.querySelectorAll('[role=switch]')].some(control => control.getAttribute('aria-label').replace(/[\\u2068\\u2069]/g, '') === '启用剪贴板历史')) throw new Error('The extension enable label was not translated');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "extension-chinese-minimum-size.png"));
    await browser("fill", "input[aria-label='最多保留条目']", "88");
    await browser(
        "eval",
        `(() => {
        window.retainedConfigInput = document.querySelector('input[aria-label="最多保留条目"]');
        window.retainedConfigGroup = window.retainedConfigInput.closest('.configuration-group');
        window.retainedResourceReads = document.documentElement.dataset.localizationReads;
        window.retainedConfigInput.setSelectionRange(1, 1);
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Maximum history entries"]');
        if (input !== window.retainedConfigInput || input.value !== '88' || input.selectionStart !== 1) throw new Error('Locale discarded the configuration edit or caret');
        if (document.activeElement !== input || input.closest('.configuration-group') !== window.retainedConfigGroup) throw new Error('Locale remounted the field group or changed focus');
        if (document.documentElement.dataset.localizationReads !== window.retainedResourceReads) throw new Error('Locale fetched configuration resources again');
        if (document.querySelector('.sidebar input').value !== 'clip') throw new Error('Locale reset the Settings query');
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "input[aria-label='最多保留条目']");
    await browser("fill", ".sidebar input", "");
    await browser("find", "role", "button", "click", "--name", "通用", "--exact");
    await browser("find", "role", "combobox", "click", "--name", "语言", "--exact");
    await browser("find", "role", "option", "click", "--name", "English", "--exact");
    await browser("wait", "--fn", "document.documentElement.lang === 'en'");
    await browser("wait", "--fn", "!document.querySelector('[role=listbox]')");
    await browser("find", "role", "combobox", "click", "--name", "Language", "--exact");
    await browser("find", "role", "option", "click", "--name", "Follow System", "--exact");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('[role=combobox][aria-label=Language]').textContent.includes('Follow System')"
    );
    await browser("set", "viewport", "920", "680");
    await _verifyProgress(browser, output);
    await _verifyConfigurationFailure(browser, output);
    await _verifyConfigurationAdmission(browser, output);
    await _verifyValidation(browser, output);
    await _verifyHostCopy(browser, output);
    await _verifyInvocationFailure(browser, output);
    await _verifyHostFailures(browser, output);
    await _verifyPreferenceFailures(browser, output);
    await _verifyResourceFailure(browser, output);
}

async function _verifyLanguageLayout(browser: BrowserCommand, output: string): Promise<void>
{
    for (const [width, height, reduced] of [[920, 680, false], [620, 520, false], [620, 520, true]] as const)
    {
        await browser("eval", "location.href = location.origin + '/?surface=settings&state=language-layout'");
        await browser("set", "viewport", String(width), String(height));
        await browser("set", "media", "dark", ...(reduced ? ["reduced-motion"] : []));
        await browser("find", "role", "button", "click", "--name", "General", "--exact");
        await browser(
            "eval",
            `document.fonts.ready.then(() => {
            const first = document.querySelector('.general-page .row');
            const language = first?.querySelector('[role=combobox][aria-label="Language"]');
            if (!language || first.closest('section').querySelector('h2')) throw new Error('Language must be the first standalone setting');
            if (language.getBoundingClientRect().bottom > innerHeight) throw new Error('Language is not immediately visible');
            return true;
        })`
        );
        for (
            const [label, option, locale] of [["Language", "Follow System", "zh-Hans"], [
                "语言",
                "English",
                "en"
            ]] as const
        )
        {
            await browser("find", "role", "combobox", "click", "--name", label, "--exact");
            await verifyScrollGutters(browser);
            await browser(
                "eval",
                `(() => {
                const trigger = document.querySelector('[role=combobox][aria-label="${label}"]');
                const controls = [...document.querySelectorAll('.general-page fieldset button')];
                const rect = element => { const {x,y,width,height} = element.getBoundingClientRect(); return {x,y,width,height}; };
                const sample = () => ({text: trigger.textContent.trim(), rect: rect(trigger), controls: controls.map(rect)});
                const state = window.languageLayout = { trigger, controls, initial: sample(), frames: [], done: false };
                const started = performance.now();
                const frame = () => {
                    state.frames.push(sample());
                    if (performance.now() - started < 700) requestAnimationFrame(frame);
                    else state.done = true;
                };
                requestAnimationFrame(frame);
                return state.initial;
            })()`
            );
            await browser("find", "role", "option", "click", "--name", option, "--exact");
            await browser(
                "wait",
                "--fn",
                `window.languageLayout.done && document.documentElement.lang === '${locale}'`
            );
            await browser(
                "eval",
                `(() => {
                const state = window.languageLayout;
                if (!state.trigger.isConnected || state.controls.some(control => !control.isConnected)) throw new Error('Language remounted a settings control');
                if (document.activeElement !== state.trigger) throw new Error('Language moved select focus');
                const last = state.frames.at(-1);
                for (const frame of state.frames) {
                    for (const key of ['x','y','width','height']) {
                        if (Math.abs(frame.rect[key] - state.initial.rect[key]) > 0.5) {
                            throw new Error('Language trigger shifted: ' + JSON.stringify({initial:state.initial, frame, last}));
                        }
                        for (let index = 0; index < state.controls.length; index++) {
                            const initial = state.initial.controls[index][key];
                            const final = last.controls[index][key];
                            const current = frame.controls[index][key];
                            if (current < Math.min(initial, final) - 0.5 || current > Math.max(initial, final) + 0.5) {
                                throw new Error('Settings control had transient reflow: ' + JSON.stringify({index, key, initial, current, final}));
                            }
                        }
                    }
                }
                if (document.documentElement.scrollWidth > innerWidth || document.body.scrollWidth > innerWidth) throw new Error('Settings overflowed horizontally');
                const text = state.trigger.querySelector('span');
                if (text.scrollWidth > text.clientWidth) throw new Error('Built-in language label was truncated');
                return {initial:state.initial, last, frames:state.frames.length};
            })()`
            );
            await browser("screenshot", join(output, `language-layout-${width}-${locale}-${reduced}.png`));
        }
    }
    await browser("set", "media", "dark");
    await browser("eval", "location.href = location.origin + '/?surface=settings'");
    await browser("set", "viewport", "920", "680");
    await browser("wait", "input[aria-label='Maximum history entries']");
}

async function _verifyLongSettingsText(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.href = location.origin + '/?surface=settings&state=long-labels'");
    await browser("set", "viewport", "620", "520");
    await browser("wait", ".extension-heading h1");
    for (const locale of ["en", "zh-Hans"])
    {
        await browser("eval", `window.changeSettingsLanguage('${locale}')`);
        await browser("wait", "--fn", `document.documentElement.lang === '${locale}'`);
        await browser(
            "eval",
            `(() => {
            const page = document.querySelector('.extension-page');
            const bad = [...page.querySelectorAll('h1,h2,h3,p,.property-copy,.property-control,.property-control button,.directory-toolbar')]
                .filter(element => element.clientWidth && element.scrollWidth > element.clientWidth + 1)
                .map(element => ({tag:element.tagName, class:element.className, width:element.clientWidth, scroll:element.scrollWidth}));
            if (bad.length) throw new Error('Long translated settings text overflowed: ' + JSON.stringify(bad));
            const columns = [...page.querySelectorAll('.property.scalar')].map(row => {
                const copy = row.querySelector('.property-copy');
                const control = row.querySelector('.property-control');
                return copy && control ? {copy:copy.getBoundingClientRect().width, control:control.getBoundingClientRect().width, row:row.clientWidth} : null;
            }).filter(Boolean);
            if (columns.some(column => column.copy < column.row / 4 || column.control > column.row / 2 + 1)) throw new Error('Long action label displaced settings copy: ' + JSON.stringify(columns));
            return {locale:document.documentElement.lang, columns};
        })()`
        );
        await browser("screenshot", join(output, `settings-long-labels-${locale}.png`));
        await browser("eval", "document.querySelector('.action-group button').scrollIntoView({block:'center'}); true");
        await browser("screenshot", join(output, `settings-long-action-${locale}.png`));
        await browser("eval", "document.querySelector('.extension-heading').scrollIntoView({block:'start'}); true");
    }
    await browser("set", "media", "dark");
    await browser("eval", "location.href = location.origin + '/?surface=settings'");
    await browser("set", "viewport", "920", "680");
    await browser("wait", "input[aria-label='Maximum history entries']");
}

async function _verifyAboutSearch(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("fill", ".sidebar input", "关于");
    await browser("wait", ".result-group[aria-label='关于']");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('.sidebar input');
        input.setSelectionRange(1, 2);
        window.retainedAboutSearch = { input, group: document.querySelector('.result-group'),
            button: document.querySelector('.result-group h2 button'),
            searches: document.documentElement.dataset.hostSearch,
            resources: document.documentElement.dataset.localizationReads };
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser("wait", ".result-group[aria-label='About']");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedAboutSearch;
        if (document.querySelector('.sidebar input') !== kept.input || kept.input.value !== '关于') throw new Error('About translation replaced the search input or query');
        if (document.activeElement !== kept.input || kept.input.selectionStart !== 1 || kept.input.selectionEnd !== 2) throw new Error('About translation lost search focus or selection');
        if (document.querySelector('.result-group') !== kept.group || document.querySelector('.result-group h2 button') !== kept.button) throw new Error('About translation remounted the search result');
        if (kept.button.textContent.trim() !== 'About') throw new Error('About result retained untranslated text');
        if (document.documentElement.dataset.hostSearch !== kept.searches || document.documentElement.dataset.localizationReads !== kept.resources) throw new Error('About translation repeated search or resource reads');
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "About", "--exact");
    await browser("wait", "--fn", "document.querySelector('.about-page h1')?.textContent === 'About'");
    await browser("screenshot", join(output, "settings-about-search-english.png"));
    await browser("eval", "window.changeSettingsLanguage('zh-Hans')");
    await browser("wait", "--fn", "document.querySelector('.about-page h1')?.textContent === '关于'");
    await browser("wait", ".result-group[aria-label='关于']");
    await browser("screenshot", join(output, "settings-about-search-chinese.png"));
    await browser("fill", ".sidebar input", "");
}

async function _verifyResourceFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=resource-error'; true");
    await browser("wait", "--text", "正在加载的翻译过多");
    await browser(
        "eval",
        `(() => {
        const error = document.querySelector('.empty-state[role=alert]');
        if (!error.textContent.includes('Original Settings resource <diagnostic>')) throw new Error('Settings resource diagnostic was lost');
        window.settingsResourceFailure = error;
        window.settingsResourceReads = document.documentElement.dataset.hostResources;
        window.settingsMetadataReads = document.documentElement.dataset.hostSettingsRead;
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-initial-resource-error-chinese.png"));
    await browser("eval", "window.changeSettingsLanguage('en')");
    await browser("wait", "--text", "Too many translations are loading.");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.empty-state[role=alert]') !== window.settingsResourceFailure) throw new Error('Language replaced initial Settings failure');
        if (document.documentElement.dataset.hostResources !== window.settingsResourceReads || document.documentElement.dataset.hostSettingsRead !== window.settingsMetadataReads) throw new Error('Language retried Settings initialization');
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "Try again", "--exact");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.hostSettingsRead !== '2' || document.documentElement.lang !== 'en') throw new Error('Explicit retry lost the latest locale or repeated initialization');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
}

async function _verifyPreferenceFailures(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=preference-errors'; true");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser("find", "role", "button", "click", "--name", "Add folder", "--exact");
    await browser("wait", "--text", "The selected folder is unavailable.");
    await browser(
        "eval",
        `(() => {
        window.failedDirectory = document.querySelector('.directories [role=alert]');
        window.directoryPickCount = document.documentElement.dataset.hostDirectoryPick;
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "所选文件夹不可用");
    await browser(
        "eval",
        `(() => {
        const error = document.querySelector('.directories [role=alert]');
        if (error !== window.failedDirectory || !error.textContent.includes('Original folder <diagnostic>')) throw new Error('Folder failure lost its diagnostic or element');
        if (document.documentElement.dataset.hostDirectoryPick !== window.directoryPickCount) throw new Error('Language reopened folder picker');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-directory-error-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "添加文件夹", "--exact");
    await browser("wait", "--text", "/Example/Added folder");
    await browser("find", "role", "button", "click", "--name", "通用", "--exact");
    await browser("find", "role", "button", "click", "--name", "打开启动器的快捷键", "--exact");
    await browser("wait", "--text", "无法更新快捷键录制状态");
    await browser(
        "eval",
        `(() => {
        window.failedRecorder = document.activeElement;
        window.recordingCalls = document.documentElement.dataset.hostRecording;
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser("wait", "--text", "Shortcut recording could not be updated.");
    await browser(
        "eval",
        `(() => {
        const recorder = document.querySelector('button[aria-label="Open launcher shortcut"]');
        if (recorder !== window.failedRecorder || document.activeElement !== recorder || recorder.getAttribute('aria-pressed') !== 'false') throw new Error('Recorder failure lost focus or state');
        if (!document.querySelector('#shortcut-help').textContent.includes('Original recording <diagnostic>')) throw new Error('Recorder lost diagnostic');
        if (document.documentElement.dataset.hostRecording !== window.recordingCalls) throw new Error('Language retried recording');
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "Open launcher shortcut", "--exact");
    await browser("wait", "--text", "Press keys…");
    await browser("press", "Control+k");
    await browser("wait", "--text", "Original confirmation <diagnostic>");
    await browser(
        "eval",
        `(() => {
        window.preferenceToast = document.querySelector('.toast');
        window.preferenceRecorder = document.querySelector('button[aria-label="Open launcher shortcut"]');
        window.preferenceDraft = window.preferenceRecorder.textContent;
        window.preferenceRecordingCalls = document.documentElement.dataset.hostRecording;
        if (!window.preferenceDraft.includes('K')) throw new Error('Missing unconfirmed shortcut draft');
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "无法释放原快捷键");
    await browser(
        "eval",
        `(() => {
        const toast = document.querySelector('.toast');
        const recorder = document.querySelector('button[aria-label="打开启动器的快捷键"]');
        if (toast !== window.preferenceToast || recorder !== window.preferenceRecorder || recorder.textContent !== window.preferenceDraft) throw new Error('Language replaced transaction failure or draft');
        for (const stage of ['release', 'rollback', 'cleanup', 'confirmation']) if (!toast.textContent.includes('Original ' + stage + ' <diagnostic>')) throw new Error('Missing recovery cause: ' + stage);
        if (document.documentElement.dataset.hostShortcutWrite !== '1' || document.documentElement.dataset.hostRecording !== window.preferenceRecordingCalls) throw new Error('Language repeated shortcut work');
        return true;
    })()`
    );
    await browser("set", "viewport", "700", "500");
    await browser("screenshot", join(output, "settings-preferences-recovery-chinese.png"));
    await browser(
        "eval",
        `(() => {
        const detail = document.querySelector('.toast span');
        const close = document.querySelector('.toast button').getBoundingClientRect();
        if (detail.clientHeight > innerHeight * .35 + 1 || close.bottom > innerHeight || close.right > innerWidth) throw new Error('Recovery diagnostics displaced dismissal');
        detail.scrollTop = detail.scrollHeight;
        if (detail.scrollHeight > detail.clientHeight && detail.scrollTop <= 0) throw new Error('Recovery diagnostics cannot be scrolled');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-preferences-recovery-detail-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
    await browser("find", "role", "button", "click", "--name", "打开启动器的快捷键", "--exact");
    await browser("wait", "--text", "请按快捷键…");
    await browser("press", "Control+k");
    await browser("wait", "--fn", "document.documentElement.dataset.hostShortcutWrite === '2'");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.toast') || window.verificationErrors.length) throw new Error('Explicit shortcut retry did not recover cleanly');
        return true;
    })()`
    );
    await browser("set", "viewport", "920", "680");
}

async function _verifyInvocationFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=invocation-error'; true");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("find", "role", "button", "click", "--name", "Clear history", "--exact");
    await browser("wait", "--fn", "document.documentElement.dataset.actionAttempts === '1'");
    await browser("eval", "window.changeSettingsLanguage('zh-Hans')");
    await browser("wait", "input[aria-label='最多保留条目']");
    await browser("eval", "window.finishInvocationFailure(); true");
    await browser("wait", "--text", "无法清空剪贴板历史");
    await browser(
        "eval",
        `(() => {
        window.retainedActionToast = document.querySelector('.toast');
        window.retainedActionReads = document.documentElement.dataset.localizationReads;
        document.querySelector('.toast button').focus();
        window.retainedActionFocus = document.activeElement;
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser("wait", "--text", "Clipboard history could not be cleared.");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.toast') !== window.retainedActionToast || document.activeElement !== window.retainedActionFocus) throw new Error('Language replaced action toast or focus');
        if (!window.retainedActionToast.textContent.includes('EACCES: /Example/历史.db')) throw new Error('Action diagnostic lost');
        if (document.documentElement.dataset.localizationReads !== window.retainedActionReads || document.documentElement.dataset.actionAttempts !== '1') throw new Error('Error translation resubmitted action or fetched resources');
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "无法清空剪贴板历史");
    await browser("screenshot", join(output, "settings-action-error-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
    await browser("find", "role", "button", "click", "--name", "清除", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("find", "role", "button", "click", "--name", "清除历史记录", "--exact");
    await browser(
        "wait",
        "--fn",
        "document.documentElement.dataset.actionAttempts === '2' && document.documentElement.dataset.confirmed === 'true'"
    );
    await browser("eval", "if (document.querySelector('.toast')) throw new Error('Retry left a failure toast'); true");
}

async function _verifyProgress(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=progress'; true");
    await browser("wait", "[role=progressbar]");
    await browser(
        "eval",
        `(() => {
        const bar = document.querySelector('[role=progressbar]');
        if (!bar.getAttribute('aria-label').includes('Scanning application sources') || bar.getAttribute('aria-valuenow') !== '1'
            || bar.getAttribute('aria-valuemax') !== '3') throw new Error('Progress snapshot was not restored');
        window.retainedProgress = { bar, field: document.querySelector('[aria-label="User applications"]'),
            reads: document.documentElement.dataset.localizationReads };
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser(
        "wait",
        "--fn",
        "document.querySelector('[role=progressbar]')?.getAttribute('aria-label').includes('正在扫描应用来源')"
    );
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedProgress;
        const bar = document.querySelector('[role=progressbar]');
        if (bar !== kept.bar || bar.getAttribute('aria-valuenow') !== '1' || bar.getAttribute('aria-valuemax') !== '3') throw new Error('Language changed progress identity or work units');
        if (!kept.field || document.querySelector('[aria-label="用户应用"]') !== kept.field || !kept.field.closest('fieldset').disabled) throw new Error('Language changed the accepted settings operation');
        if (document.documentElement.dataset.localizationReads !== kept.reads) throw new Error('Warm progress translation fetched resources');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-progress-chinese.png"));
    await browser("eval", "window.publishForeignProgress()");
    await browser("wait", "--text", "Progress resources do not belong to the operation owner.");
    await browser(
        "eval",
        `(() => {
        const bar = document.querySelector('[role=progressbar]');
        if (bar !== window.retainedProgress.bar || bar.getAttribute('aria-valuenow') !== '1'
            || !bar.getAttribute('aria-label').includes('正在扫描应用来源')) throw new Error('Invalid progress replaced the last complete presentation');
        return true;
    })()`
    );
    await browser("eval", "window.finishSettingsProgress()");
    await browser("wait", "--fn", "!document.querySelector('[role=progressbar]')");
    await browser(
        "eval",
        `(() => {
        const field = document.querySelector('[aria-label="用户应用"]');
        if (field !== window.retainedProgress.field || field.closest('fieldset').disabled || field.getAttribute('aria-checked') !== 'false') throw new Error('Completion did not preserve the accepted value');
        if (document.documentElement.dataset.progressCompleted !== 'true') throw new Error('Missing terminal delivery');
        return true;
    })()`
    );
}

async function _verifyConfigurationFailure(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=progress'; true");
    await browser("wait", "[role=progressbar]");
    await browser(
        "eval",
        `window.pendingConfiguration = {
        field: document.querySelector('[aria-label="User applications"]'),
        bar: document.querySelector('[role=progressbar]'),
        writes: document.documentElement.dataset.settingsWrites
    }; window.changeSettingsLanguage('zh-Hans')`
    );
    await browser(
        "wait",
        "--fn",
        "document.querySelector('[role=progressbar]')?.getAttribute('aria-label').includes('正在扫描应用来源')"
    );
    await browser("eval", "window.changeSettingsLanguage('en')");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('[role=progressbar]')?.getAttribute('aria-label').includes('Scanning application sources')"
    );
    await browser(
        "eval",
        `(() => {
        const kept = window.pendingConfiguration;
        if (document.querySelector('[role=progressbar]') !== kept.bar || document.querySelector('[aria-label="User applications"]') !== kept.field
            || !kept.field.closest('fieldset').disabled || document.documentElement.dataset.settingsWrites !== kept.writes) throw new Error('Locale changed pending configuration ownership');
        return true;
    })()`
    );
    await browser(
        "eval",
        "window.finishSettingsProgress('applicationFailure')"
    );
    await browser("wait", "--text", "The change was saved but could not be applied.");
    await browser("wait", "--text", "Application settings could not be applied.");
    await browser("wait", "--fn", "!document.querySelector('[role=progressbar]')");
    await browser(
        "eval",
        `(() => {
        window.retainedFailure = { toast: document.querySelector('.toast'),
            field: document.querySelector('[aria-label="User applications"]'),
            reads: document.documentElement.dataset.localizationReads, writes: document.documentElement.dataset.settingsWrites };
        document.querySelector('.toast button').focus();
        window.retainedFailure.focus = document.activeElement;
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "更改已保存，但无法应用。");
    await browser("wait", "--text", "无法完成应用设置的更新。");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedFailure;
        const toast = document.querySelector('.toast');
        const field = document.querySelector('[aria-label="用户应用"]');
        if (toast !== kept.toast || !toast.textContent.includes('device permission denied <original>')) throw new Error('Locale replaced the error or diagnostic detail');
        if (field !== kept.field || field.closest('fieldset').disabled || field.getAttribute('aria-checked') !== 'false') throw new Error('Locale changed terminal configuration state');
        if (document.activeElement !== kept.focus || kept.focus.getAttribute('aria-label') !== '关闭通知') throw new Error('Error translation lost focus or accessible dismissal');
        if (document.documentElement.dataset.localizationReads !== kept.reads) throw new Error('Error translation fetched extension resources');
        if (document.documentElement.dataset.settingsWrites !== kept.writes) throw new Error('Error translation repeated configuration application');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-error-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
    await browser("wait", "--fn", "!document.querySelector('.toast')");
    await browser("find", "role", "switch", "click", "--name", "用户应用", "--exact");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('[aria-label=用户应用]')?.getAttribute('aria-checked') === 'true'"
    );
    await browser(
        "eval",
        `(() => {
        const field = document.querySelector('[aria-label="用户应用"]');
        if (field !== window.retainedFailure.field || field.closest('fieldset').disabled || document.querySelector('.toast')
            || document.documentElement.dataset.settingsWrites !== '1') throw new Error('Explicit edit did not recover exactly once');
        return true;
    })()`
    );
}

async function _verifyValidation(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=validation'; true");
    await browser("wait", "input[aria-label='Sample count']");
    await browser("fill", "input[aria-label='Sample count']", "-");
    await browser("press", "Tab");
    await browser("wait", "--text", "Enter a whole number.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Sample count"]');
        input.focus(); input.setSelectionRange(1, 1);
        window.retainedValidation = { input, toast: document.querySelector('.toast'), reads: document.documentElement.dataset.localizationReads };
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser(
        "wait",
        "--fn",
        "document.querySelector('input[aria-label=\"Sample count\"]').validationMessage.includes('请输入整数')"
    );
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedValidation;
        const input = document.querySelector('input[aria-label="Sample count"]');
        if (input !== kept.input || input.value !== '-' || input.selectionStart !== 1 || document.activeElement !== input) throw new Error('Locale replaced the invalid draft or caret');
        if (input.getAttribute('aria-invalid') !== 'true' || !input.validity.customError) throw new Error('Locale cleared validation state');
        if (document.querySelector('.toast') !== kept.toast || !kept.toast.textContent.includes('请输入整数')) throw new Error('Validation notification did not update in place');
        if (Number(document.documentElement.dataset.settingsWrites ?? 0) !== 0) throw new Error('Invalid draft entered the operation queue');
        if (document.documentElement.dataset.localizationReads !== kept.reads) throw new Error('Validation translation fetched resources');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-validation-chinese.png"));
    await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
    await browser("fill", "input[aria-label='Sample count']", "6");
    await browser("press", "Tab");
    await browser("wait", "--fn", "document.documentElement.dataset.settingsWrites === '1'");
    await browser("fill", "input[aria-label='Sample text']", "你好");
    await browser("press", "Tab");
    await browser("wait", "--fn", "document.querySelector('.toast')?.textContent.includes('UTF-8')");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Sample text"]');
        input.focus(); input.setSelectionRange(1, 1);
        window.retainedValidationText = input;
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser(
        "wait",
        "--fn",
        "document.querySelector('input[aria-label=\"Sample text\"]').validationMessage.includes('Enter at most')"
    );
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Sample text"]');
        if (input !== window.retainedValidationText || input.value !== '你好' || input.selectionStart !== 1 || document.activeElement !== input) throw new Error('Language changed the original UTF-8 draft');
        if (document.documentElement.dataset.settingsWrites !== '1') throw new Error('Oversized text entered the operation queue');
        return true;
    })()`
    );
    await browser("fill", "input[aria-label='Sample text']", "ok");
    await browser("press", "Tab");
    await browser("find", "role", "button", "click", "--name", "Dismiss notification", "--exact");
    await browser("find", "role", "button", "click", "--name", "Show more");
    await browser("wait", "input[id='setting-fixture.options-20-count']");
    await browser("fill", "input[id='setting-fixture.options-20-count']", "0");
    await browser("find", "role", "button", "click", "--name", "Add entry", "--exact");
    await browser("wait", "--fn", "document.querySelector('.toast')?.textContent.includes('Entry')");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[id="setting-fixture.options-20-count"]');
        input.focus(); input.setSelectionRange(1, 1);
        window.retainedArrayValidation = { input, group: document.querySelector('.array'), writes: document.documentElement.dataset.settingsWrites };
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--fn", "document.querySelector('.toast')?.textContent.includes('个条目')");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedArrayValidation;
        if (document.querySelector('.array') !== kept.group || document.querySelector('input[id="setting-fixture.options-20-count"]') !== kept.input || kept.input.value !== '0') throw new Error('Locale discarded the expanded array draft');
        if (document.activeElement !== kept.input || kept.input.selectionStart !== 1) throw new Error('Locale moved nested field focus');
        if (!document.querySelector('.toast').textContent.includes('21') || !document.querySelector('.toast').textContent.includes('count')) throw new Error('Nested error lost its exact field path');
        if (!kept.group.textContent.includes('添加条目') || !kept.group.textContent.includes('移除') || !kept.group.textContent.includes('包含')) throw new Error('Structured field controls did not translate');
        if (document.documentElement.dataset.settingsWrites !== kept.writes || kept.writes !== '1') throw new Error('Invalid array was saved');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-nested-validation-chinese.png"));
}

async function _verifyHostCopy(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&platform=windows'; true");
    await browser("wait", "button[aria-label='Minimize']");
    await browser("find", "role", "button", "click", "--name", "General", "--exact");
    await browser("find", "role", "button", "click", "--name", "Open launcher shortcut", "--exact");
    await browser("wait", "--text", "Press keys…");
    await browser(
        "eval",
        `(() => {
        window.retainedRecorder = document.activeElement;
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "请按快捷键…");
    await browser(
        "eval",
        `(() => {
        const recorder = document.querySelector('button[aria-label="打开启动器的快捷键"]');
        if (recorder !== window.retainedRecorder || document.activeElement !== recorder || recorder.getAttribute('aria-pressed') !== 'true') throw new Error('Locale reset shortcut recording');
        for (const label of ['最小化', '最大化', '关闭设置']) if (!document.querySelector('button[aria-label="' + label + '"]')) throw new Error('Window control label missing: ' + label);
        if (document.querySelector('.drag-region').textContent.trim() !== '设置') throw new Error('Caption title did not translate');
        return true;
    })()`
    );
    await browser("press", "Escape");
    await browser("find", "role", "button", "click", "--name", "关于", "--exact");
    await browser("wait", "[aria-label='应用信息']");
    await browser(
        "wait",
        "--fn",
        `(() => {
        const target = document.querySelector('.sidebar button.active')?.getBoundingClientRect();
        const highlight = document.querySelector('.sidebar .selection-highlight')?.getBoundingClientRect();
        return target && highlight && Math.abs(target.top - highlight.top) < 1;
    })()`
    );
    await browser(
        "eval",
        `(() => {
        if (!document.querySelector('.about-page').textContent.includes('版本') || !document.querySelector('.about-page').textContent.includes('应用、命令和扩展的统一入口。')) throw new Error('About copy did not translate');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "about-chinese-windows-layout.png"));
    await browser("find", "role", "button", "click", "--name", "剪贴板历史", "--exact");
    await browser("wait", "--text", "运行中");
    await browser("wait", "button[aria-label='移除文件夹']");
    await browser(
        "eval",
        `(() => {
        const remove = document.querySelector('button[aria-label="移除文件夹"]');
        const description = document.getElementById(remove.getAttribute('aria-describedby'));
        if (!description || description.textContent !== '/Example/Clipboard') throw new Error('Localized folder action lost its original path description');
        if (document.documentElement.scrollWidth > innerWidth) throw new Error('Chinese Settings overflowed');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "folders-chinese.png"));
}

async function _verifyLifecycle(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "window.publishLifecycleFailure('restarting')");
    await browser("wait", "--text", "One automatic restart is in progress.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Maximum history entries"]');
        input.focus(); input.setSelectionRange(0, 1);
        const alert = document.querySelector('.lifecycle-error');
        alert.scrollTop = 80;
        window.retainedLifecycle = { input, value: input.value, alert, reads: document.documentElement.dataset.localizationReads,
            writes: document.documentElement.dataset.settingsWrites, changes: document.documentElement.dataset.lifecycleChanges };
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "正在进行一次自动重启");
    await browser(
        "eval",
        `(() => {
        const kept = window.retainedLifecycle;
        if (document.activeElement !== kept.input || kept.input.value !== kept.value || kept.input.selectionStart !== 0 || kept.input.selectionEnd !== 1) throw new Error('Lifecycle localization lost edit state');
        if (document.querySelector('.lifecycle-error') !== kept.alert || kept.alert.scrollTop !== 80) throw new Error('Lifecycle localization reset diagnostics');
        if (!kept.alert.textContent.includes('剪贴板历史设置无效') || !kept.alert.textContent.includes('stderr: original diagnostic 199')) throw new Error('Lifecycle cause or original output missing');
        if (!document.querySelector('section[aria-labelledby="extension-enabled-title"] [role=switch]').matches(':disabled')) throw new Error('Pending lifecycle control is not disabled');
        if (document.documentElement.dataset.localizationReads !== kept.reads || document.documentElement.dataset.settingsWrites !== kept.writes || document.documentElement.dataset.lifecycleChanges !== kept.changes) throw new Error('Lifecycle localization performed backend work');
        return window.publishLifecycleFailure('restartLimitReached');
    })()`
    );
    await browser("wait", "--text", "已停止自动重启");
    await browser(
        "eval",
        `(() => {
        const alert = document.querySelector('.lifecycle-error');
        alert.scrollTop = 0;
        if (!alert.textContent.includes('first process stderr') || !alert.textContent.includes('重启失败')) throw new Error('Recovery dropped a cause');
        if (alert.getBoundingClientRect().height > innerHeight * .35 + 1 || alert.scrollHeight <= alert.clientHeight) throw new Error('Lifecycle diagnostic is unbounded');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "lifecycle-failure-chinese.png"));
    await browser("click", "section[aria-labelledby='extension-enabled-title'] [role=switch]");
    await browser("eval", "window.changeSettingsLanguage('en')");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.dataset.lifecycleChanges !== '1') throw new Error('Language change repeated lifecycle request');
        window.finishLifecycleFailure(); return true;
    })()`
    );
    await browser("wait", "--text", "The extension's enabled setting could not be saved.");
    await browser("eval", "window.changeSettingsLanguage('zh-Hans')");
    await browser("wait", "--text", "无法保存扩展的启用设置");
    await browser(
        "eval",
        `(() => {
        if (!document.querySelector('.toast').textContent.includes('EACCES: /Example/config.json')) throw new Error('Lifecycle command lost diagnostic');
        if (document.documentElement.dataset.lifecycleChanges !== '1') throw new Error('Retained error retried lifecycle change');
        return true;
    })()`
    );
    await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
    await browser("click", "section[aria-labelledby='extension-enabled-title'] [role=switch]");
    await browser("click", "section[aria-labelledby='extension-enabled-title'] [role=switch]");
    await browser("wait", "--text", "运行中");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.lifecycle-error') || document.documentElement.dataset.lifecycleChanges !== '3') throw new Error('Explicit reenable did not recover exactly once');
        return window.changeSettingsLanguage('en');
    })()`
    );
}

async function _verifyConfigurationAdmission(browser: BrowserCommand, output: string): Promise<void>
{
    for (const code of ["rejected", "unconfirmed"])
    {
        await browser("eval", "location.search = '?surface=settings'; true");
        await browser("wait", "input[aria-label='Maximum history entries']");
        await browser("eval", `window.failNextSettingsWrite('${code}'); true`);
        await browser("fill", "input[aria-label='Maximum history entries']", "88");
        await browser("click", "input[aria-label='Maximum age in days']");
        await browser("wait", "--text", code === "rejected" ? "The change was not accepted" : "It is unclear whether");
        await browser(
            "eval",
            `(() => {
            const input = document.querySelector('input[aria-label="Maximum history entries"]');
            if (input.value !== '88' || input.closest('fieldset').disabled) throw new Error('Failure discarded or disabled the draft');
            input.focus(); input.setSelectionRange(1, 2);
            window.retainedAdmission = { input, toast: document.querySelector('.toast'), reads: document.documentElement.dataset.localizationReads };
            return window.changeSettingsLanguage('zh-Hans');
        })()`
        );
        await browser("wait", "--text", code === "rejected" ? "此更改未被接受" : "无法确认更改是否已保存或生效");
        await browser(
            "eval",
            `(() => {
            const kept = window.retainedAdmission;
            if (!kept.input.isConnected || kept.input.value !== '88' || kept.input.selectionStart !== 1 || kept.input.selectionEnd !== 2
                || document.activeElement !== kept.input) throw new Error('Language change lost the rejected edit or selection');
            if (document.querySelector('.toast') !== kept.toast || !kept.toast.textContent.includes('fixture original diagnostic <retained>')) throw new Error('Failure cause was flattened or replaced');
            if (document.documentElement.dataset.settingsWrites !== '1' || document.documentElement.dataset.localizationReads !== kept.reads) throw new Error('Presentation retried configuration work');
            if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
            return true;
        })()`
        );
        await browser("screenshot", join(output, `configuration-${code}-chinese.png`));
        await browser("find", "role", "button", "click", "--name", "关闭通知", "--exact");
        await browser("eval", "window.changeSettingsLanguage('en')");
        await browser("wait", "input[aria-label='Maximum history entries']");
        await browser(
            "eval",
            `(() => {
            if (document.documentElement.dataset.settingsWrites !== '1' || window.retainedAdmission.input.value !== '88') throw new Error('Dismissal implicitly retried or discarded the draft');
            return true;
        })()`
        );
        await browser("fill", "input[aria-label='Maximum history entries']", "89");
        await browser("click", "input[aria-label='Maximum age in days']");
        await browser("wait", "--fn", "document.documentElement.dataset.settingsWrites === '2'");
        await browser(
            "eval",
            `(() => {
            if (window.retainedAdmission.input.value !== '89' || document.querySelector('.toast')) throw new Error('Explicit configuration retry failed');
            return true;
        })()`
        );
    }
}

async function _verifyHostFailures(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=host-errors&platform=windows'; true");
    await browser("wait", "--text", "Original window update <diagnostic>");
    await browser("find", "role", "button", "click", "--name", "General", "--exact");
    await browser("wait", "--text", "Original startup read <diagnostic>");
    await browser(
        "eval",
        `(() => {
        window.hostFailureStartup = document.querySelector('[role=switch][aria-label="Launch at login"]');
        window.hostFailureRead = document.querySelector('.startup-error');
        window.hostFailureInitialReads = document.documentElement.dataset.hostStartupRead;
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "无法读取登录时启动状态");
    await browser(
        "eval",
        `(() => {
        if (document.querySelector('.startup-error') !== window.hostFailureRead || !window.hostFailureRead.textContent.includes('Original startup read <diagnostic>')) throw new Error('Startup read error lost identity or diagnostics');
        if (document.documentElement.dataset.hostStartupRead !== window.hostFailureInitialReads || document.documentElement.dataset.hostReady !== '1') throw new Error('Language retried a host operation');
        return true;
    })()`
    );
    await browser("set", "viewport", "700", "500");
    await browser("scrollintoview", ".startup-error");
    await browser("screenshot", join(output, "settings-host-read-errors-chinese.png"));
    await browser("click", ".window-error button");
    await browser("wait", "button[aria-label='关闭设置']");
    await browser("eval", "window.setStartupReadFailure(false); true");
    await browser("scrollintoview", ".startup-error + button");
    await browser(
        "wait",
        "--fn",
        "(() => { const button = document.querySelector('.startup-error + button'); const box = button.getBoundingClientRect(); return box.top >= 0 && box.bottom <= innerHeight && document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)?.closest('button') === button; })()"
    );
    await browser("click", ".startup-error + button");
    await browser(
        "eval",
        "({ reads: document.documentElement.dataset.hostStartupRead, writes: document.documentElement.dataset.hostStartupWrite, error: document.querySelector('.startup-error')?.textContent })"
    );
    await browser("wait", "--fn", "!document.querySelector('.startup-error')");
    await browser("find", "role", "switch", "click", "--name", "登录时启动", "--exact");
    await browser("wait", "--text", "Original startup write <diagnostic>");
    await browser("wait", "--text", "Original startup read <diagnostic>");
    await browser(
        "eval",
        `(() => {
        window.hostFailureToast = document.querySelector('.toast');
        window.hostFailureRead = document.querySelector('.startup-error');
        window.hostFailureWriteReads = document.documentElement.dataset.hostStartupRead;
        return window.changeSettingsLanguage('en');
    })()`
    );
    await browser("wait", "--text", "The launch-at-login change could not be confirmed");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('[role=switch][aria-label="Launch at login"]');
        if (input !== window.hostFailureStartup || input.getAttribute('aria-checked') !== 'true') throw new Error('Unknown startup outcome lost the draft');
        if (document.querySelector('.toast') !== window.hostFailureToast || document.querySelector('.startup-error') !== window.hostFailureRead) throw new Error('Language replaced either startup failure');
        if (document.documentElement.dataset.hostStartupRead !== window.hostFailureWriteReads || document.documentElement.dataset.hostStartupWrite !== '1') throw new Error('Language retried startup work');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-startup-errors.png"));
    await browser("eval", "window.setStartupReadFailure(false); true");
    await browser("scrollintoview", ".startup-error + button");
    await browser(
        "wait",
        "--fn",
        "(() => { const button = document.querySelector('.startup-error + button'); const box = button.getBoundingClientRect(); return box.top >= 0 && box.bottom <= innerHeight && document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)?.closest('button') === button; })()"
    );
    await browser("click", ".startup-error + button");
    await browser(
        "eval",
        "({ reads: document.documentElement.dataset.hostStartupRead, writes: document.documentElement.dataset.hostStartupWrite, error: document.querySelector('.startup-error')?.textContent })"
    );
    await browser("wait", "--fn", "!document.querySelector('.startup-error')");
    await browser("eval", "window.hostFailureRecoveredReads = document.documentElement.dataset.hostStartupRead; true");
    await browser("find", "role", "button", "click", "--name", "Close Settings", "--exact");
    await browser("wait", "--text", "The window could not be hidden.");
    await browser("fill", "input[aria-label='Search settings']", "clip");
    await browser("wait", "--text", "Settings could not be searched.");
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Search settings"]');
        input.focus(); input.setSelectionRange(1, 3);
        window.hostFailureSearch = input;
        return window.changeSettingsLanguage('zh-Hans');
    })()`
    );
    await browser("wait", "--text", "无法搜索设置");
    await browser(
        "eval",
        `(() => {
        const input = window.hostFailureSearch;
        if (document.activeElement !== input || input.value !== 'clip' || input.selectionStart !== 1 || input.selectionEnd !== 3) throw new Error('Settings failure lost search input state');
        if (!document.querySelector('.window-error').textContent.includes('无法隐藏窗口') || !document.querySelector('.window-error').textContent.includes('Original close <diagnostic>')) throw new Error('Window failure lost translated explanation or diagnostics');
        if (document.documentElement.dataset.hostSearch !== '1' || document.documentElement.dataset.hostWindow !== '1' || document.documentElement.dataset.hostStartupRead !== window.hostFailureRecoveredReads) throw new Error('Language reran a failed host operation');
        if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; '));
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-window-search-errors-chinese.png"));
    await browser("click", "nav .search-message + button");
    await browser("wait", "--fn", "!document.querySelector('nav .search-message[role=alert]')");
    await browser("set", "viewport", "920", "680");
}

async function _verifyScrollLayout(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("eval", "location.search = '?surface=settings&state=scroll-layout'; true");
    await browser("set", "viewport", "620", "520");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser("fill", ".sidebar input", "Clipboard");
    await browser("wait", ".sidebar .result-field");
    await browser("find", "role", "button", "click", "--name", "Syntax highlighting", "--exact");
    await browser("hover", ".sidebar .result-field.active");
    await verifyScrollGutters(browser);
    await browser(
        "eval",
        `(() => {
        const sidebar = document.querySelector('.sidebar .scroll-viewport');
        if (sidebar.scrollHeight <= sidebar.clientHeight) throw new Error('Sidebar scrollbar fixture does not overflow');
        const highlight = document.querySelector('.sidebar .selection-highlight');
        const rail = sidebar.closest('.scroll-area').querySelector('.scroll-track');
        if (highlight.getBoundingClientRect().right > rail.getBoundingClientRect().left - 2) throw new Error('Sidebar highlight overlaps scrollbar');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-sidebar-scrollbar.png"));
    await browser("scrollintoview", ".directory-items");
    await browser("hover", ".directory-items .directory:first-child");
    await verifyScrollGutters(browser);
    await browser("screenshot", join(output, "settings-directory-scrollbar.png"));
    await browser("eval", "window.changeSettingsLanguage('zh-Hans')");
    await browser("wait", "--fn", "document.documentElement.lang === 'zh-Hans'");
    await browser("set", "viewport", "920", "680");
    await verifyScrollGutters(browser);
    await browser("eval", "location.search = '?surface=settings'; true");
    await browser("wait", "input[aria-label='Maximum history entries']");
}
