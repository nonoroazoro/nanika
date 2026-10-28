import type { SyntaxLanguage } from "../../src/syntax/SyntaxLanguage";

/**
 * Prose, diagnostics and incomplete markers that resemble supported languages.
 * Keep every language represented, including formats that are never auto-detected.
 */
export const languageNegativeSamples: Record<SyntaxLanguage, string[]> = {
    c: ["#include everyone in the discussion", "#pragma remember to call", "int count (number of items)"],
    cpp: ["std::runtime_error: connection failed", "template <your name here>", "Use std::vector for storage."],
    csharp: [
        "using System resources wisely",
        "using System; then restart",
        "namespace Example is reserved",
        "The using System; directive imports a namespace."
    ],
    css: ["Expected: an object { values: [...] }", "The result is { value: 42 }", "Example:\nbody { color: red; }"],
    dockerfile: ["from here", "FROM support", "ARG please read this\nFROM support"],
    go: ["package delivered", "package main\nDelivery is scheduled for tomorrow."],
    html: ["<html-like> is a placeholder", "<html is the opening tag", "Use <div>Hello</div> as the example."],
    ini: ["[INFO]\nConnection failed\nretry = later", "[section]\nThis is a note\nkey = value"],
    java: ["import settings; then restart", "package example; delivery tomorrow", "Use import java.util.List; here."],
    javascript: [
        "Set const enabled = true in the file.",
        "This function render() displays the view.",
        "function render() displays the view.",
        "Failure:\nconst value = 1;"
    ],
    json: ["{message: hello}", "[INFO] worker started", '{"enabled": true} trailing explanation'],
    jsonc: ['// comment\n{"enabled": true,}', '{ /* settings */ "enabled": true }'],
    jsx: ["Example: const view = <Widget />;", "Use function View() { return <div>Hello</div>; } here."],
    just: [
        "set minimum-version := please update",
        "[private]\nThis is a note.",
        'Use set shell := ["bash", "-cu"] here.'
    ],
    less: ["@alice: please review this", "@color: red is the example value"],
    lua: ["local function calls are faster", "local price = the current estimate", "Example:\nlocal message = 'hello'"],
    markdown: ["#tag", "# Reminder", "ordinary_text_with_underscores", "[INFO] connection established"],
    "objective-c": ["@interface between two services", "@class starts tomorrow", "#import <your file here>"],
    php: ["<?php-like> is a placeholder", "Use <?php to open the file."],
    powershell: ["Get-Started with the installation guide", "Write-Down your notes", "param(name) means a parameter"],
    python: [
        "The signature is def greet(name):",
        "Example:\ndef greet(name):\n    return name",
        "def greet(name): is an example"
    ],
    ruby: ["def means definition\nend of explanation", "Example:\ndef greet(name)\n  puts name\nend"],
    rust: ["Example:\nfn main() {}", "use paths::to::describe the problem", "pub struct Example is a type"],
    scss: ["@include everyone in the meeting", "$budget: approximately five hundred", "@use the new settings"],
    shell: ["#!/usr/bin/env rust-script\necho hello", "Run #!/bin/bash at the top of the file."],
    sql: [
        "Select a value from the list.",
        "select item from list",
        "Please insert into the appropriate field.",
        "CREATE TABLE is documented here",
        "UPDATE users SET their preferences",
        "The query is SELECT name FROM users;"
    ],
    svelte: [
        "Use {#if visible} to conditionally render content.",
        "The <svelte:window> element handles events.",
        "Example: <input bind:value={name} />"
    ],
    swift: ["import Foundation for the example", "func greet(name: String) is a signature"],
    toml: ["[settings]\nenabled = true\nPlease restart the app.", 'name = "Example"\nversion = 1\nThis is a note.'],
    tsx: [
        "Example: const view: JSX.Element = <Widget />;",
        "The interface View { node: JSX.Element } uses <div>Hello</div>."
    ],
    typescript: [
        "The interface Entry { name: string } describes a record.",
        "Use type Name = string here.",
        "Example:\ninterface Entry { name: string }"
    ],
    xml: [
        "<?xml-like> is a placeholder",
        "<!DOCTYPE instructions are described here",
        'Use <config xmlns="urn:example" /> here.'
    ],
    yaml: [
        "Content-Length: 123\nContent-Type: application/json",
        "Error: 404\nMessage: Not Found",
        "code: 404\nmessage: Not Found",
        "Error:\n  Message: connection failed\nTry again later.",
        "status: true story\nmessage: see above",
        "Details:\n- name: sample\nPlease review the output."
    ]
};
