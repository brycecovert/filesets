# Filesets

Your swiss-army knife for dealing with identical files. 

Filesets makes it easy to clean up files with identical contents. Common use cases include: 
* Which documents are shared from my laptop backup and desktop backup?
* I have five full backups. Remove duplicated files.
* Don't remove the duplicated files, create symlinks to the older snapshots (similar to Time Machine)

## Terminology

Filesets categorizes files from multiple directories into the following categories:

* Uniques: This file's contents exists only once across all scanned directories.
* Duplicates: This file's contents exist more than once across all scanned directories.
* Firsts: This is the first instance of this file's contents across all scanned directories
* Replicas: This file's contents exist more than once across all scanned directories, and this is not the first one.

To see these categorizations in context, imagine that we scanned the following directories:
```
/backups/1/documents/a
/backups/1/documents/b

/backups/2/docs/a
```

filesets would categorize these files as follows:
```
/backups/1/documents/a [duplicate, first]
/backups/1/documents/b [unique, first]

/backups/2/docs/a [duplicate, replica]
```

The power behind this is that you can have filesets create a plan to cleanup these backups, replacing
`/backups/2/docs/a` with a symlink to `/backups/1/documents/a`.

## Usage

Seeing unique files in a single directory:
```
$ filesets -u ~/Documents
```

Seeing duplicated files in a single directory:
```
$ filesets -d ~/Documents
```

Seeing first files in a single directory:
```
$ filesets -f ~/Documents
```

Seeing replicated files in a single directory:
```
$ filesets -r ~/Documents
```

Seeing replicated files in a multiple directories:
```
$ filesets -r ~/Documents /Volumes/Backup/1/Documents
```

Seeing a plan to symlink files in priority order :
```
$ filesets -p /Volumes/Backup/1/Documents /Volumes/Backup/2/Documents
```


