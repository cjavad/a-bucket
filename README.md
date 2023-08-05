# a-bucket

> See DEPLOYMENT.md for information regarding deployment and nessary hosting requirements.

Author: Javad Asgari Shafique, Web UI by: Ole Frederik
Category: Web

## Problem statement

Given the source code of a custom CDN service, find a way to manipulate the service via network requests to get the flag, as it uses quite a lot of custom logic some deep dive or luck is required.

## Flavor text

An image gallery with a bucket, can you find the flag?

<link_to_website_here>

## Difficulty

Easy-To-Medium, estimated solve time between 30 to 1h 30 min.

## Basic approach

Use the webpage to expose the cdn service with the following source code, find clues in the configuration files and reverse engineer the protocol to manipulate the files and get the flag.

See solution in `solve.sh`, but i expect people to solve it in different ways.
