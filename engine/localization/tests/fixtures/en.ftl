-brand-name = Nanika
-greeting = Hello, { $person }!
brand = Welcome to { -brand-name }.
salutation = { -greeting(person: "Ada") }
integer = Count: { $count }
files = { $count ->
    [0] No files
    [one] One file
   *[other] { $count } files
    }
platform = { $platform ->
    [macos] Empty Trash
   *[windows] Empty Recycle Bin
    }
content = Content: { $value }
reference = { integer }
files-more = { $name } and { $count } more
