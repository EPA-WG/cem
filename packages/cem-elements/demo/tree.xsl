<!doctype html>
<html lang="en">
<body>
<template type="text/cem-ml">
    {article @class=demo-card |
        {h2 | {$datadom.attributes.title ?? "DCE with external XSLT template"}}
        {p | {slot | Hi}}
        {details @style="padding:0 1rem" @open=open |
            {summary |
                {b @style="color:green" | datadom}
                {code @style="margin-left:1rem;color:brown" | title="{$datadom.attributes.title ?? ""}"}
                {code @style="margin-left:1rem;color:brown" | data-fruit="{$datadom.attributes.data-fruit ?? ""}"}
                {code @style="margin-left:1rem;color:brown" | data-smile="{$datadom.attributes.data-smile ?? ""}"}
                {code @style="margin-left:1rem;color:brown" | data-basket="{$datadom.attributes.data-basket ?? ""}"}
            }
            {p | payload: {$datadom.payload.text}}
        }
    }
</template>
</body>
</html>
