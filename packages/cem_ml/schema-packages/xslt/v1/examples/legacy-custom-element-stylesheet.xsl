<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <article>
      <xsl:if test="$ready">
        <button>Continue</button>
      </xsl:if>
    </article>
  </xsl:template>
</xsl:stylesheet>
