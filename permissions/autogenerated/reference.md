## Default Permission

Default permissions for the plugin

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`fs-stream:allow-open-read-file-stream`

</td>
<td>

Enables the open_read_file_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:deny-open-read-file-stream`

</td>
<td>

Denies the open_read_file_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:allow-open-read-text-file-lines-stream`

</td>
<td>

Enables the open_read_text_file_lines_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:deny-open-read-text-file-lines-stream`

</td>
<td>

Denies the open_read_text_file_lines_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:allow-open-write-file-stream`

</td>
<td>

Enables the open_write_file_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:deny-open-write-file-stream`

</td>
<td>

Denies the open_write_file_stream command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fs-stream:scope`

</td>
<td>

An empty permission you can use to modify the global scope.

## Example

```json
{
  "permissions": [
    "fs-stream:allow-open-read-file-stream",
    {
      "identifier": "fs-stream:scope",
      "allow": [
        "$APPDATA/documents/**/*"
      ],
      "deny": [
        "$APPDATA/documents/secret.txt"
      ]
    }
  ]
}
```


</td>
</tr>
</table>
